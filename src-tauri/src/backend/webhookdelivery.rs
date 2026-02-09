// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Webhook delivery system for cloud event notifications.
//! Port of Go's pkg/webhookdelivery/.
//!
//! Provides types for webhook events, client configuration,
//! and service lifecycle management.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

// ---- Constants ----

/// WebSocket write timeout.
pub const WRITE_WAIT: Duration = Duration::from_secs(10);

/// WebSocket pong read timeout.
pub const PONG_WAIT: Duration = Duration::from_secs(60);

/// WebSocket ping interval (must be < PONG_WAIT).
pub const PING_PERIOD: Duration = Duration::from_secs(50);

/// Maximum WebSocket message size (10MB).
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Initial reconnect delay.
pub const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Maximum reconnect delay.
pub const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(120);

/// Reconnect backoff multiplier.
pub const RECONNECT_BACKOFF_RATE: f64 = 2.0;

// ---- Types ----

/// A webhook event received from the cloud service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event_type: String,
    pub provider: String,
    pub terminal_id: String,
    pub command: String,
    #[serde(default)]
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

/// Webhook configuration type (mirrors wconfig).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub subscribed_terminals: Vec<String>,
}

impl WebhookConfig {
    /// Check if a terminal is subscribed for webhook events.
    pub fn is_terminal_subscribed(&self, terminal_id: &str) -> bool {
        if self.subscribed_terminals.is_empty() {
            return true; // Empty list = subscribe all
        }
        self.subscribed_terminals.iter().any(|t| t == terminal_id)
    }
}

/// Webhook client state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Disconnected,
    Connecting,
    Connected,
}

/// Webhook service status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookServiceStatus {
    pub enabled: bool,
    pub connected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub terminal_count: usize,
}

/// Webhook service managing the client and terminal mappings.
pub struct WebhookService {
    config: WebhookConfig,
    terminal_map: Mutex<HashMap<String, String>>,
    state: Mutex<ClientState>,
}

impl WebhookService {
    /// Create a new webhook service with the given config.
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            config,
            terminal_map: Mutex::new(HashMap::new()),
            state: Mutex::new(ClientState::Disconnected),
        }
    }

    /// Check if the service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the current client state.
    pub fn client_state(&self) -> ClientState {
        *self.state.lock().unwrap()
    }

    /// Set the client state.
    pub fn set_client_state(&self, state: ClientState) {
        *self.state.lock().unwrap() = state;
    }

    /// Register a terminal→block mapping.
    pub fn register_terminal(&self, terminal_id: &str, block_id: &str) {
        self.terminal_map
            .lock()
            .unwrap()
            .insert(terminal_id.to_string(), block_id.to_string());
    }

    /// Unregister a terminal mapping.
    pub fn unregister_terminal(&self, terminal_id: &str) {
        self.terminal_map.lock().unwrap().remove(terminal_id);
    }

    /// Look up block ID for a terminal.
    pub fn get_block_id(&self, terminal_id: &str) -> Option<String> {
        self.terminal_map
            .lock()
            .unwrap()
            .get(terminal_id)
            .cloned()
    }

    /// Check if a terminal is subscribed per config.
    pub fn is_terminal_subscribed(&self, terminal_id: &str) -> bool {
        self.config.is_terminal_subscribed(terminal_id)
    }

    /// Get service status.
    pub fn get_status(&self) -> WebhookServiceStatus {
        WebhookServiceStatus {
            enabled: self.config.enabled,
            connected: self.client_state() == ClientState::Connected,
            workspace_id: self.config.workspace_id.clone(),
            endpoint: self.config.endpoint.clone(),
            terminal_count: self.terminal_map.lock().unwrap().len(),
        }
    }

    /// Calculate reconnect delay with exponential backoff.
    pub fn calc_reconnect_delay(attempt: u32) -> Duration {
        let delay_ms = INITIAL_RECONNECT_DELAY.as_millis() as f64
            * RECONNECT_BACKOFF_RATE.powi(attempt as i32);
        let capped = delay_ms.min(MAX_RECONNECT_DELAY.as_millis() as f64);
        Duration::from_millis(capped as u64)
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_serde() {
        let event = WebhookEvent {
            event_type: "command".to_string(),
            provider: "github".to_string(),
            terminal_id: "term-1".to_string(),
            command: "deploy".to_string(),
            timestamp: 1700000000000,
            workspace_id: Some("ws-1".to_string()),
            raw_data: None,
            connection_id: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\":\"command\""));
        assert!(!json.contains("raw_data")); // None skipped

        let parsed: WebhookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_type, "command");
        assert_eq!(parsed.provider, "github");
        assert_eq!(parsed.terminal_id, "term-1");
    }

    #[test]
    fn test_webhook_event_from_json() {
        let json = r#"{"event_type":"push","provider":"gitlab","terminal_id":"t1","command":"test","timestamp":0,"workspace_id":"ws","raw_data":"raw","connection_id":"conn"}"#;
        let event: WebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "push");
        assert_eq!(event.raw_data.as_deref(), Some("raw"));
        assert_eq!(event.connection_id.as_deref(), Some("conn"));
    }

    #[test]
    fn test_webhook_config_default() {
        let config = WebhookConfig::default();
        assert!(!config.enabled);
        assert!(config.endpoint.is_none());
        assert!(config.subscribed_terminals.is_empty());
    }

    #[test]
    fn test_webhook_config_subscribe_all() {
        let config = WebhookConfig {
            enabled: true,
            subscribed_terminals: vec![],
            ..Default::default()
        };
        // Empty list means subscribe all
        assert!(config.is_terminal_subscribed("any-terminal"));
    }

    #[test]
    fn test_webhook_config_subscribe_specific() {
        let config = WebhookConfig {
            enabled: true,
            subscribed_terminals: vec!["term-1".to_string(), "term-2".to_string()],
            ..Default::default()
        };
        assert!(config.is_terminal_subscribed("term-1"));
        assert!(config.is_terminal_subscribed("term-2"));
        assert!(!config.is_terminal_subscribed("term-3"));
    }

    #[test]
    fn test_service_new() {
        let service = WebhookService::new(WebhookConfig {
            enabled: true,
            endpoint: Some("wss://example.com".to_string()),
            ..Default::default()
        });

        assert!(service.is_enabled());
        assert_eq!(service.client_state(), ClientState::Disconnected);
    }

    #[test]
    fn test_service_terminal_mapping() {
        let service = WebhookService::new(WebhookConfig::default());

        service.register_terminal("term-1", "block-a");
        service.register_terminal("term-2", "block-b");

        assert_eq!(
            service.get_block_id("term-1").as_deref(),
            Some("block-a")
        );
        assert_eq!(
            service.get_block_id("term-2").as_deref(),
            Some("block-b")
        );
        assert!(service.get_block_id("term-3").is_none());

        service.unregister_terminal("term-1");
        assert!(service.get_block_id("term-1").is_none());
    }

    #[test]
    fn test_service_state_transitions() {
        let service = WebhookService::new(WebhookConfig::default());

        assert_eq!(service.client_state(), ClientState::Disconnected);

        service.set_client_state(ClientState::Connecting);
        assert_eq!(service.client_state(), ClientState::Connecting);

        service.set_client_state(ClientState::Connected);
        assert_eq!(service.client_state(), ClientState::Connected);
    }

    #[test]
    fn test_service_status() {
        let service = WebhookService::new(WebhookConfig {
            enabled: true,
            endpoint: Some("wss://example.com".to_string()),
            workspace_id: Some("ws-1".to_string()),
            ..Default::default()
        });

        service.register_terminal("t1", "b1");
        service.set_client_state(ClientState::Connected);

        let status = service.get_status();
        assert!(status.enabled);
        assert!(status.connected);
        assert_eq!(status.workspace_id.as_deref(), Some("ws-1"));
        assert_eq!(status.terminal_count, 1);
    }

    #[test]
    fn test_reconnect_delay_exponential() {
        let d0 = WebhookService::calc_reconnect_delay(0);
        let d1 = WebhookService::calc_reconnect_delay(1);
        let d2 = WebhookService::calc_reconnect_delay(2);
        let d10 = WebhookService::calc_reconnect_delay(10);

        assert_eq!(d0, Duration::from_secs(1));
        assert_eq!(d1, Duration::from_secs(2));
        assert_eq!(d2, Duration::from_secs(4));
        // Capped at MAX_RECONNECT_DELAY
        assert!(d10 <= MAX_RECONNECT_DELAY);
    }

    #[test]
    fn test_service_status_serde() {
        let status = WebhookServiceStatus {
            enabled: true,
            connected: false,
            workspace_id: Some("ws-1".to_string()),
            endpoint: Some("wss://example.com".to_string()),
            terminal_count: 3,
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: WebhookServiceStatus = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert!(!parsed.connected);
        assert_eq!(parsed.terminal_count, 3);
    }
}
