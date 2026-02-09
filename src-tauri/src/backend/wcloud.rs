// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! WCloud client: telemetry submission to the WaveMux cloud API.
//! Port of Go's pkg/wcloud/.

use crate::backend::telemetry::{ActivityType, TEvent};

use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

// ---- Constants ----

pub const WCLOUD_ENDPOINT: &str = "https://api.waveterm.dev/central";
pub const WCLOUD_ENDPOINT_VAR_NAME: &str = "WCLOUD_ENDPOINT";
pub const WCLOUD_WS_ENDPOINT: &str = "wss://wsapi.waveterm.dev/";
pub const WCLOUD_WS_ENDPOINT_VAR_NAME: &str = "WCLOUD_WS_ENDPOINT";

pub const API_VERSION: i32 = 1;
pub const MAX_PTY_UPDATE_SIZE: usize = 128 * 1024;
pub const MAX_UPDATES_PER_REQ: usize = 10;
pub const MAX_UPDATES_TO_DEDUP: usize = 1000;
pub const MAX_UPDATE_WRITER_ERRORS: i32 = 3;
pub const MAX_UPDATE_PAYLOAD_SIZE: usize = 1024 * 1024;

pub const WCLOUD_DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
pub const WCLOUD_WEB_SHARE_UPDATE_TIMEOUT: Duration = Duration::from_secs(15);

pub const TELEMETRY_URL: &str = "/telemetry";
pub const TEVENTS_URL: &str = "/tevents";
pub const NO_TELEMETRY_URL: &str = "/no-telemetry";
pub const WEB_SHARE_UPDATE_URL: &str = "/auth/web-share-update";

pub const TEVENTS_BATCH_SIZE: usize = 200;
pub const TEVENTS_MAX_BATCHES: usize = 10;

// ---- Cached endpoint overrides ----

static ENDPOINT_CACHE: OnceLock<String> = OnceLock::new();
static WS_ENDPOINT_CACHE: OnceLock<String> = OnceLock::new();

// ---- Request/Response types ----

/// Input payload for sending telemetry events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TEventsInputType {
    pub clientid: String,
    pub events: Vec<TEvent>,
}

/// Input payload for the no-telemetry preference update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoTelemetryInputType {
    pub clientid: String,
    pub value: bool,
}

/// Input payload for sending legacy activity telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryInputType {
    pub userid: String,
    pub clientid: String,
    #[serde(rename = "apptype", skip_serializing_if = "Option::is_none")]
    pub app_type: Option<String>,
    #[serde(rename = "autoupdateenabled", skip_serializing_if = "Option::is_none")]
    pub auto_update_enabled: Option<bool>,
    #[serde(rename = "autoupdatechannel", skip_serializing_if = "Option::is_none")]
    pub auto_update_channel: Option<String>,
    pub curday: String,
    pub activity: Vec<ActivityType>,
}

// ---- Endpoint management ----

/// Validate and cache WCloud endpoint environment variables.
/// Removes the env vars after caching for security.
pub fn cache_and_remove_env_vars() -> Result<(), String> {
    // Cache HTTP endpoint
    if let Ok(endpoint) = env::var(WCLOUD_ENDPOINT_VAR_NAME) {
        if !endpoint.is_empty() {
            check_endpoint_var(&endpoint, "WCloudEndpoint", WCLOUD_ENDPOINT_VAR_NAME)?;
            let _ = ENDPOINT_CACHE.set(endpoint);
            env::remove_var(WCLOUD_ENDPOINT_VAR_NAME);
        }
    }

    // Cache WebSocket endpoint
    if let Ok(ws_endpoint) = env::var(WCLOUD_WS_ENDPOINT_VAR_NAME) {
        if !ws_endpoint.is_empty() {
            check_ws_endpoint_var(&ws_endpoint, "WCloudWSEndpoint", WCLOUD_WS_ENDPOINT_VAR_NAME)?;
            let _ = WS_ENDPOINT_CACHE.set(ws_endpoint);
            env::remove_var(WCLOUD_WS_ENDPOINT_VAR_NAME);
        }
    }

    Ok(())
}

/// Get the WCloud HTTP API endpoint.
pub fn get_endpoint() -> &'static str {
    ENDPOINT_CACHE.get().map_or(WCLOUD_ENDPOINT, |s| s.as_str())
}

/// Get the WCloud WebSocket endpoint.
pub fn get_ws_endpoint() -> &'static str {
    WS_ENDPOINT_CACHE
        .get()
        .map_or(WCLOUD_WS_ENDPOINT, |s| s.as_str())
}

/// Validate an HTTP endpoint URL format.
fn check_endpoint_var(endpoint: &str, debug_name: &str, var_name: &str) -> Result<(), String> {
    if !endpoint.starts_with("https://") && !endpoint.starts_with("http://") {
        return Err(format!(
            "{} ({}) must start with https:// or http://",
            debug_name, var_name
        ));
    }
    if endpoint.ends_with('/') {
        return Err(format!(
            "{} ({}) must not end with /",
            debug_name, var_name
        ));
    }
    Ok(())
}

/// Validate a WebSocket endpoint URL format.
fn check_ws_endpoint_var(endpoint: &str, debug_name: &str, var_name: &str) -> Result<(), String> {
    if !endpoint.starts_with("wss://") && !endpoint.starts_with("ws://") {
        return Err(format!(
            "{} ({}) must start with wss:// or ws://",
            debug_name, var_name
        ));
    }
    Ok(())
}

/// Build the full URL for a WCloud API endpoint path.
pub fn build_url(path: &str) -> String {
    format!("{}{}", get_endpoint(), path)
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(WCLOUD_ENDPOINT, "https://api.waveterm.dev/central");
        assert_eq!(WCLOUD_WS_ENDPOINT, "wss://wsapi.waveterm.dev/");
        assert_eq!(API_VERSION, 1);
        assert_eq!(MAX_PTY_UPDATE_SIZE, 131072);
        assert_eq!(TEVENTS_BATCH_SIZE, 200);
        assert_eq!(TEVENTS_MAX_BATCHES, 10);
    }

    #[test]
    fn test_url_constants() {
        assert_eq!(TELEMETRY_URL, "/telemetry");
        assert_eq!(TEVENTS_URL, "/tevents");
        assert_eq!(NO_TELEMETRY_URL, "/no-telemetry");
        assert_eq!(WEB_SHARE_UPDATE_URL, "/auth/web-share-update");
    }

    #[test]
    fn test_get_endpoint_default() {
        // Without env var override, should return default
        let endpoint = get_endpoint();
        assert!(endpoint.starts_with("https://"));
    }

    #[test]
    fn test_get_ws_endpoint_default() {
        let ws = get_ws_endpoint();
        assert!(ws.starts_with("wss://"));
    }

    #[test]
    fn test_build_url() {
        let url = build_url("/telemetry");
        assert!(url.ends_with("/telemetry"));
        assert!(url.starts_with("https://") || url.starts_with("http://"));
    }

    #[test]
    fn test_check_endpoint_var_valid() {
        assert!(check_endpoint_var("https://api.example.com", "test", "TEST_VAR").is_ok());
        assert!(check_endpoint_var("http://localhost:8080", "test", "TEST_VAR").is_ok());
    }

    #[test]
    fn test_check_endpoint_var_invalid_scheme() {
        assert!(check_endpoint_var("ws://example.com", "test", "TEST_VAR").is_err());
        assert!(check_endpoint_var("ftp://example.com", "test", "TEST_VAR").is_err());
    }

    #[test]
    fn test_check_endpoint_var_trailing_slash() {
        assert!(check_endpoint_var("https://api.example.com/", "test", "TEST_VAR").is_err());
    }

    #[test]
    fn test_check_ws_endpoint_var_valid() {
        assert!(check_ws_endpoint_var("wss://ws.example.com/", "test", "TEST_VAR").is_ok());
        assert!(check_ws_endpoint_var("ws://localhost:8080/", "test", "TEST_VAR").is_ok());
    }

    #[test]
    fn test_check_ws_endpoint_var_invalid() {
        assert!(check_ws_endpoint_var("https://example.com", "test", "TEST_VAR").is_err());
    }

    #[test]
    fn test_tevents_input_serde() {
        let input = TEventsInputType {
            clientid: "client-123".to_string(),
            events: vec![],
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""clientid":"client-123""#));
        assert!(json.contains(r#""events":[]"#));

        let deser: TEventsInputType = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.clientid, "client-123");
        assert!(deser.events.is_empty());
    }

    #[test]
    fn test_no_telemetry_input_serde() {
        let input = NoTelemetryInputType {
            clientid: "client-456".to_string(),
            value: true,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""clientid":"client-456""#));
        assert!(json.contains(r#""value":true"#));

        let deser: NoTelemetryInputType = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.clientid, "client-456");
        assert!(deser.value);
    }

    #[test]
    fn test_telemetry_input_serde() {
        let input = TelemetryInputType {
            userid: "user-1".to_string(),
            clientid: "client-1".to_string(),
            app_type: Some("wavemux".to_string()),
            auto_update_enabled: Some(true),
            auto_update_channel: Some("stable".to_string()),
            curday: "2024-03-15".to_string(),
            activity: vec![],
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""userid":"user-1""#));
        assert!(json.contains(r#""clientid":"client-1""#));
        assert!(json.contains(r#""apptype":"wavemux""#));
        assert!(json.contains(r#""autoupdateenabled":true"#));
        assert!(json.contains(r#""curday":"2024-03-15""#));
    }

    #[test]
    fn test_telemetry_input_optional_fields() {
        let input = TelemetryInputType {
            userid: "u".to_string(),
            clientid: "c".to_string(),
            app_type: None,
            auto_update_enabled: None,
            auto_update_channel: None,
            curday: "2024-01-01".to_string(),
            activity: vec![],
        };
        let json = serde_json::to_string(&input).unwrap();
        // Optional fields should be omitted
        assert!(!json.contains("apptype"));
        assert!(!json.contains("autoupdateenabled"));
        assert!(!json.contains("autoupdatechannel"));
    }

    #[test]
    fn test_timeout_constants() {
        assert_eq!(WCLOUD_DEFAULT_TIMEOUT, Duration::from_secs(5));
        assert_eq!(WCLOUD_WEB_SHARE_UPDATE_TIMEOUT, Duration::from_secs(15));
    }
}
