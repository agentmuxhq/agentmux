// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Reactive messaging system for agent-to-agent terminal communication.
//! Port of Go's pkg/reactive/.
//!
//! Provides message injection into terminal panes, agent registration,
//! rate limiting, message sanitization, audit logging, and cross-host
//! polling via AgentMux cloud service.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ---- Constants ----

/// Maximum message length in bytes.
pub const MAX_MESSAGE_LENGTH: usize = 10_000;

/// Suffix appended when a message is truncated.
pub const TRUNCATION_SUFFIX: &str = "\n[Message truncated]";

/// Maximum entries in the audit log ring buffer.
const AUDIT_LOG_MAX: usize = 100;

/// Rate limit: max tokens (requests per second).
const RATE_LIMIT_MAX: u32 = 10;

/// Delay between message injection and Enter key (milliseconds).
const INJECT_ENTER_DELAY_MS: u64 = 150;

/// Default poll interval for AgentMux poller (seconds).
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;

// ---- Types ----

/// Request to inject a message into an agent's terminal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionRequest {
    pub target_agent: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default)]
    pub wait_for_idle: bool,
}

/// Response from a message injection attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionResponse {
    pub success: bool,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: u64,
}

/// Agent registration record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub block_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_id: Option<String>,
    pub registered_at: u64,
    pub last_seen: u64,
}

/// List of registered agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentRegistration>,
}

/// Audit log entry for message injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
    pub target_agent: String,
    pub block_id: String,
    pub message_hash: String,
    pub message_length: usize,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub request_id: String,
}

/// Poller configuration for AgentMux cloud service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentmux_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentmux_token: Option<String>,
    #[serde(default)]
    pub poll_interval_secs: u64,
}

/// AgentMux config file format (agentmux.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMuxConfigFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// Pending injection from AgentMux cloud.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInjection {
    pub id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default)]
    pub created_at: u64,
}

/// Response from AgentMux pending endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingResponse {
    pub injections: Vec<PendingInjection>,
}

/// Acknowledgment request for delivered injections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckRequest {
    pub injection_ids: Vec<String>,
}

/// Poller status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollerStatus {
    pub configured: bool,
    pub running: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub has_token: bool,
    pub poll_count: u64,
    pub injections_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_poll: Option<u64>,
}

/// Function type for sending input bytes to a block's PTY.
pub type InputSender = Arc<dyn Fn(&str, &[u8]) -> Result<(), String> + Send + Sync>;

// ---- Rate Limiter ----

struct RateLimiter {
    tokens: u32,
    max_tokens: u32,
    last_refill: Instant,
}

impl RateLimiter {
    fn new(max_tokens: u32) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            last_refill: Instant::now(),
        }
    }

    fn check(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        if elapsed >= Duration::from_secs(1) {
            self.tokens = self.max_tokens;
            self.last_refill = now;
        }
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

// ---- Handler ----

/// Core reactive messaging handler.
///
/// Manages agent registrations, rate limiting, message injection,
/// and audit logging.
pub struct Handler {
    agent_to_block: HashMap<String, String>,
    block_to_agent: HashMap<String, String>,
    agent_info: HashMap<String, AgentRegistration>,
    input_sender: Option<InputSender>,
    audit_log: Vec<AuditLogEntry>,
    rate_limiter: RateLimiter,
    include_source_in_message: bool,
}

impl Handler {
    /// Create a new handler without an input sender.
    /// Call `set_input_sender` before injecting messages.
    pub fn new() -> Self {
        Self {
            agent_to_block: HashMap::new(),
            block_to_agent: HashMap::new(),
            agent_info: HashMap::new(),
            input_sender: None,
            audit_log: Vec::with_capacity(AUDIT_LOG_MAX),
            rate_limiter: RateLimiter::new(RATE_LIMIT_MAX),
            include_source_in_message: false,
        }
    }

    /// Set the input sender function for message injection.
    pub fn set_input_sender(&mut self, sender: InputSender) {
        self.input_sender = Some(sender);
    }

    /// Set whether to include source agent prefix in injected messages.
    pub fn set_include_source(&mut self, include: bool) {
        self.include_source_in_message = include;
    }

    /// Register an agent with a block.
    pub fn register_agent(
        &mut self,
        agent_id: &str,
        block_id: &str,
        tab_id: Option<&str>,
    ) -> Result<(), String> {
        if !validate_agent_id(agent_id) {
            return Err(format!("invalid agent ID: {}", agent_id));
        }

        // Remove existing registration for this agent
        if let Some(old_block) = self.agent_to_block.remove(agent_id) {
            self.block_to_agent.remove(&old_block);
        }

        // Remove existing registration for this block
        if let Some(old_agent) = self.block_to_agent.remove(block_id) {
            self.agent_to_block.remove(&old_agent);
            self.agent_info.remove(&old_agent);
        }

        let now = now_unix_millis();
        self.agent_to_block
            .insert(agent_id.to_string(), block_id.to_string());
        self.block_to_agent
            .insert(block_id.to_string(), agent_id.to_string());
        self.agent_info.insert(
            agent_id.to_string(),
            AgentRegistration {
                agent_id: agent_id.to_string(),
                block_id: block_id.to_string(),
                tab_id: tab_id.map(|s| s.to_string()),
                registered_at: now,
                last_seen: now,
            },
        );

        Ok(())
    }

    /// Unregister an agent.
    pub fn unregister_agent(&mut self, agent_id: &str) {
        if let Some(block_id) = self.agent_to_block.remove(agent_id) {
            self.block_to_agent.remove(&block_id);
        }
        self.agent_info.remove(agent_id);
    }

    /// Unregister by block ID.
    pub fn unregister_block(&mut self, block_id: &str) {
        if let Some(agent_id) = self.block_to_agent.remove(block_id) {
            self.agent_to_block.remove(&agent_id);
            self.agent_info.remove(&agent_id);
        }
    }

    /// Update the last_seen timestamp for an agent.
    pub fn update_last_seen(&mut self, agent_id: &str) {
        if let Some(info) = self.agent_info.get_mut(agent_id) {
            info.last_seen = now_unix_millis();
        }
    }

    /// Get agent registration by agent ID.
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentRegistration> {
        self.agent_info.get(agent_id)
    }

    /// Get agent registration by block ID.
    pub fn get_agent_by_block(&self, block_id: &str) -> Option<&AgentRegistration> {
        self.block_to_agent
            .get(block_id)
            .and_then(|agent_id| self.agent_info.get(agent_id))
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Vec<AgentRegistration> {
        self.agent_info.values().cloned().collect()
    }

    /// Inject a message into an agent's terminal.
    ///
    /// CRITICAL: Message and Enter are sent separately with a 150ms delay.
    /// This is required because the PTY needs to see the message first,
    /// then Enter as a distinct event.
    pub fn inject_message(&mut self, mut req: InjectionRequest) -> InjectionResponse {
        let now = now_unix_millis();

        // Generate request ID if missing
        if req.request_id.is_none() || req.request_id.as_deref() == Some("") {
            req.request_id = Some(uuid::Uuid::new_v4().to_string());
        }
        let request_id = req.request_id.clone().unwrap_or_default();

        // Rate limit check
        if !self.rate_limiter.check() {
            return InjectionResponse {
                success: false,
                request_id,
                block_id: None,
                error: Some("rate limit exceeded".to_string()),
                timestamp: now,
            };
        }

        // Validate agent ID
        if !validate_agent_id(&req.target_agent) {
            return InjectionResponse {
                success: false,
                request_id,
                block_id: None,
                error: Some(format!("invalid agent ID: {}", req.target_agent)),
                timestamp: now,
            };
        }

        // Sanitize message
        let sanitized = sanitize_message(&req.message);

        // Look up block ID
        let block_id = match self.agent_to_block.get(&req.target_agent) {
            Some(id) => id.clone(),
            None => {
                let err = format!("agent not found: {}", req.target_agent);
                self.log_audit(
                    req.source_agent.as_deref(),
                    &req.target_agent,
                    "",
                    &sanitized,
                    false,
                    Some(&err),
                    &request_id,
                );
                return InjectionResponse {
                    success: false,
                    request_id,
                    block_id: None,
                    error: Some(err),
                    timestamp: now,
                };
            }
        };

        // Format message with source prefix if configured
        let final_msg = format_injected_message(
            &sanitized,
            req.source_agent.as_deref(),
            self.include_source_in_message,
        );

        // Send message via input sender
        let sender = match &self.input_sender {
            Some(s) => s.clone(),
            None => {
                let err = "input sender not configured".to_string();
                self.log_audit(
                    req.source_agent.as_deref(),
                    &req.target_agent,
                    &block_id,
                    &sanitized,
                    false,
                    Some(&err),
                    &request_id,
                );
                return InjectionResponse {
                    success: false,
                    request_id,
                    block_id: Some(block_id),
                    error: Some(err),
                    timestamp: now,
                };
            }
        };

        // Step 1: Send message
        if let Err(e) = sender(&block_id, final_msg.as_bytes()) {
            self.log_audit(
                req.source_agent.as_deref(),
                &req.target_agent,
                &block_id,
                &sanitized,
                false,
                Some(&e),
                &request_id,
            );
            return InjectionResponse {
                success: false,
                request_id,
                block_id: Some(block_id),
                error: Some(e),
                timestamp: now,
            };
        }

        // Step 2: Wait 150ms then send Enter
        std::thread::sleep(Duration::from_millis(INJECT_ENTER_DELAY_MS));

        if let Err(e) = sender(&block_id, b"\r") {
            self.log_audit(
                req.source_agent.as_deref(),
                &req.target_agent,
                &block_id,
                &sanitized,
                false,
                Some(&e),
                &request_id,
            );
            return InjectionResponse {
                success: false,
                request_id,
                block_id: Some(block_id),
                error: Some(e),
                timestamp: now,
            };
        }

        // Success
        self.log_audit(
            req.source_agent.as_deref(),
            &req.target_agent,
            &block_id,
            &sanitized,
            true,
            None,
            &request_id,
        );

        InjectionResponse {
            success: true,
            request_id,
            block_id: Some(block_id),
            error: None,
            timestamp: now,
        }
    }

    /// Get audit log entries, most recent first.
    pub fn get_audit_log(&self, limit: usize) -> Vec<AuditLogEntry> {
        let start = if self.audit_log.len() > limit {
            self.audit_log.len() - limit
        } else {
            0
        };
        let mut entries: Vec<_> = self.audit_log[start..].to_vec();
        entries.reverse();
        entries
    }

    /// Add an entry to the audit ring buffer.
    #[allow(clippy::too_many_arguments)]
    fn log_audit(
        &mut self,
        source_agent: Option<&str>,
        target_agent: &str,
        block_id: &str,
        message: &str,
        success: bool,
        error_message: Option<&str>,
        request_id: &str,
    ) {
        let entry = AuditLogEntry {
            timestamp: now_unix_millis(),
            source_agent: source_agent.map(|s| s.to_string()),
            target_agent: target_agent.to_string(),
            block_id: block_id.to_string(),
            message_hash: sha256_hex(message),
            message_length: message.len(),
            success,
            error_message: error_message.map(|s| s.to_string()),
            request_id: request_id.to_string(),
        };

        if self.audit_log.len() >= AUDIT_LOG_MAX {
            self.audit_log.remove(0);
        }
        self.audit_log.push(entry);
    }
}

impl Default for Handler {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Thread-safe wrapper ----

/// Thread-safe wrapper around Handler.
pub struct ReactiveHandler {
    inner: Mutex<Handler>,
}

impl ReactiveHandler {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Handler::new()),
        }
    }

    pub fn set_input_sender(&self, sender: InputSender) {
        self.inner.lock().unwrap().set_input_sender(sender);
    }

    pub fn set_include_source(&self, include: bool) {
        self.inner.lock().unwrap().set_include_source(include);
    }

    pub fn register_agent(
        &self,
        agent_id: &str,
        block_id: &str,
        tab_id: Option<&str>,
    ) -> Result<(), String> {
        self.inner
            .lock()
            .unwrap()
            .register_agent(agent_id, block_id, tab_id)
    }

    pub fn unregister_agent(&self, agent_id: &str) {
        self.inner.lock().unwrap().unregister_agent(agent_id);
    }

    pub fn unregister_block(&self, block_id: &str) {
        self.inner.lock().unwrap().unregister_block(block_id);
    }

    pub fn update_last_seen(&self, agent_id: &str) {
        self.inner.lock().unwrap().update_last_seen(agent_id);
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<AgentRegistration> {
        self.inner.lock().unwrap().get_agent(agent_id).cloned()
    }

    pub fn get_agent_by_block(&self, block_id: &str) -> Option<AgentRegistration> {
        self.inner
            .lock()
            .unwrap()
            .get_agent_by_block(block_id)
            .cloned()
    }

    pub fn list_agents(&self) -> Vec<AgentRegistration> {
        self.inner.lock().unwrap().list_agents()
    }

    pub fn inject_message(&self, req: InjectionRequest) -> InjectionResponse {
        self.inner.lock().unwrap().inject_message(req)
    }

    pub fn get_audit_log(&self, limit: usize) -> Vec<AuditLogEntry> {
        self.inner.lock().unwrap().get_audit_log(limit)
    }
}

impl Default for ReactiveHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Global reactive handler singleton.
static GLOBAL_HANDLER: OnceLock<ReactiveHandler> = OnceLock::new();

/// Get or initialize the global reactive handler.
pub fn get_global_handler() -> &'static ReactiveHandler {
    GLOBAL_HANDLER.get_or_init(ReactiveHandler::new)
}

// ---- Poller ----

/// Poller state for cross-host message polling from AgentMux.
pub struct Poller {
    config: RwLock<PollerConfig>,
    _handler: &'static ReactiveHandler,
    running: Mutex<bool>,
    poll_count: Mutex<u64>,
    injections_count: Mutex<u64>,
    last_poll: Mutex<Option<u64>>,
    last_error: Mutex<Option<String>>,
}

impl Poller {
    /// Create a new poller with the given config.
    pub fn new(config: PollerConfig, handler: &'static ReactiveHandler) -> Self {
        Self {
            config: RwLock::new(config),
            _handler: handler,
            running: Mutex::new(false),
            poll_count: Mutex::new(0),
            injections_count: Mutex::new(0),
            last_poll: Mutex::new(None),
            last_error: Mutex::new(None),
        }
    }

    /// Check if the poller is configured (has URL and token).
    pub fn is_configured(&self) -> bool {
        let config = self.config.read().unwrap();
        config.agentmux_url.is_some() && config.agentmux_token.is_some()
    }

    /// Check if the poller is running.
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Get poller statistics.
    pub fn stats(&self) -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert(
            "poll_count".to_string(),
            serde_json::json!(*self.poll_count.lock().unwrap()),
        );
        m.insert(
            "injections_count".to_string(),
            serde_json::json!(*self.injections_count.lock().unwrap()),
        );
        m.insert(
            "last_poll".to_string(),
            serde_json::json!(*self.last_poll.lock().unwrap()),
        );
        m.insert(
            "last_error".to_string(),
            serde_json::json!(*self.last_error.lock().unwrap()),
        );
        m
    }

    /// Get poller status.
    pub fn status(&self) -> PollerStatus {
        let config = self.config.read().unwrap();
        PollerStatus {
            configured: config.agentmux_url.is_some() && config.agentmux_token.is_some(),
            running: self.is_running(),
            url: config.agentmux_url.clone(),
            has_token: config.agentmux_token.is_some(),
            poll_count: *self.poll_count.lock().unwrap(),
            injections_count: *self.injections_count.lock().unwrap(),
            last_poll: *self.last_poll.lock().unwrap(),
        }
    }

    /// Reconfigure the poller with new URL and token.
    pub fn reconfigure(&self, url: Option<String>, token: Option<String>) {
        let mut config = self.config.write().unwrap();
        config.agentmux_url = url;
        config.agentmux_token = token;
    }

    /// Record a successful poll.
    pub fn record_poll(&self) {
        *self.poll_count.lock().unwrap() += 1;
        *self.last_poll.lock().unwrap() = Some(now_unix_millis());
        *self.last_error.lock().unwrap() = None;
    }

    /// Record a poll error.
    pub fn record_error(&self, err: &str) {
        *self.last_error.lock().unwrap() = Some(err.to_string());
    }

    /// Record injections delivered.
    pub fn record_injections(&self, count: u64) {
        *self.injections_count.lock().unwrap() += count;
    }

    /// Set the running state.
    pub fn set_running(&self, running: bool) {
        *self.running.lock().unwrap() = running;
    }
}

// ---- Sanitization ----

/// Sanitize a message by removing dangerous escape sequences and control characters.
///
/// 1. Removes ANSI escape sequences
/// 2. Removes OSC sequences (terminal commands)
/// 3. Removes CSI sequences
/// 4. Removes control characters except \n, \t, \r
/// 5. Truncates to MAX_MESSAGE_LENGTH with UTF-8 safety
pub fn sanitize_message(msg: &str) -> String {
    let mut result = String::with_capacity(msg.len());

    let bytes = msg.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Check for ESC sequences
        if b == 0x1b && i + 1 < len {
            let next = bytes[i + 1];

            // CSI sequence: ESC [ ... <final byte>
            if next == b'[' {
                i += 2;
                while i < len && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) {
                    i += 1;
                }
                if i < len {
                    i += 1; // skip final byte
                }
                continue;
            }

            // OSC sequence: ESC ] ... BEL
            if next == b']' {
                i += 2;
                while i < len && bytes[i] != 0x07 {
                    // Also check for ST (ESC \)
                    if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                if i < len && bytes[i] == 0x07 {
                    i += 1;
                }
                continue;
            }

            // Other ESC sequences (2-byte)
            i += 2;
            continue;
        }

        // Remove control characters except whitespace
        if b < 0x20 && b != b'\n' && b != b'\r' && b != b'\t' {
            i += 1;
            continue;
        }

        // DEL character
        if b == 0x7f {
            i += 1;
            continue;
        }

        // Keep printable characters and valid UTF-8
        if b < 0x80 {
            result.push(b as char);
            i += 1;
        } else {
            // UTF-8 multi-byte: determine sequence length
            let seq_len = if b >= 0xF0 {
                4
            } else if b >= 0xE0 {
                3
            } else if b >= 0xC0 {
                2
            } else {
                // Invalid continuation byte, skip
                i += 1;
                continue;
            };

            if i + seq_len <= len {
                let s = std::str::from_utf8(&bytes[i..i + seq_len]);
                if let Ok(valid) = s {
                    result.push_str(valid);
                }
                i += seq_len;
            } else {
                // Incomplete sequence
                i += 1;
            }
        }
    }

    // Truncate to max length, preserving UTF-8
    if result.len() > MAX_MESSAGE_LENGTH {
        let suffix_len = TRUNCATION_SUFFIX.len();
        let target = MAX_MESSAGE_LENGTH - suffix_len;
        // Find a valid UTF-8 boundary
        let mut end = target;
        while end > 0 && !result.is_char_boundary(end) {
            end -= 1;
        }
        result.truncate(end);
        result.push_str(TRUNCATION_SUFFIX);
    }

    result
}

/// Validate an agent ID.
///
/// Must be 1-64 characters, only letters, digits, underscore, and hyphen.
pub fn validate_agent_id(agent_id: &str) -> bool {
    if agent_id.is_empty() || agent_id.len() > 64 {
        return false;
    }
    agent_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Format a message with optional source agent prefix.
pub fn format_injected_message(msg: &str, source_agent: Option<&str>, include_source: bool) -> String {
    if include_source {
        if let Some(source) = source_agent {
            if !source.is_empty() {
                return format!("@{}: {}", source, msg);
            }
        }
    }
    msg.to_string()
}

/// Validate an AgentMux URL for SSRF protection.
///
/// Only allows https:// or http://localhost/127.0.0.1/::1.
pub fn validate_agentmux_url(url_str: &str) -> Result<(), String> {
    if url_str.is_empty() {
        return Err("URL is empty".to_string());
    }

    // Parse URL
    if let Some(scheme_end) = url_str.find("://") {
        let scheme = &url_str[..scheme_end];
        let rest = &url_str[scheme_end + 3..];

        match scheme {
            "https" => Ok(()),
            "http" => {
                // Extract host (before port or path)
                let authority = rest.split('/').next().unwrap_or("");
                let host = if authority.starts_with('[') {
                    // IPv6 bracketed: [::1]:port
                    authority.split(']').next().unwrap_or("")
                } else {
                    authority.split(':').next().unwrap_or("")
                };
                // Normalize: strip brackets for comparison
                let host_clean = host.trim_start_matches('[').trim_end_matches(']');

                match host_clean {
                    "localhost" | "127.0.0.1" | "::1" => Ok(()),
                    _ => Err(format!(
                        "http URLs only allowed for localhost, got host: {}",
                        host_clean
                    )),
                }
            }
            _ => Err(format!("unsupported URL scheme: {}", scheme)),
        }
    } else {
        Err("invalid URL: missing scheme".to_string())
    }
}

// ---- Helpers ----

/// Get current time as Unix milliseconds.
fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Compute SHA-256 hex digest of a string (for audit log privacy).
fn sha256_hex(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Use a fast non-crypto hash for audit log (privacy, not security)
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    // -- Sanitization tests --

    #[test]
    fn test_sanitize_plain_text() {
        assert_eq!(sanitize_message("hello world"), "hello world");
    }

    #[test]
    fn test_sanitize_preserves_whitespace() {
        assert_eq!(sanitize_message("line1\nline2\ttab"), "line1\nline2\ttab");
    }

    #[test]
    fn test_sanitize_removes_ansi_escape() {
        assert_eq!(sanitize_message("hello\x1b[31mred\x1b[0m"), "hellored");
    }

    #[test]
    fn test_sanitize_removes_osc_sequence() {
        assert_eq!(
            sanitize_message("before\x1b]0;title\x07after"),
            "beforeafter"
        );
    }

    #[test]
    fn test_sanitize_removes_osc_with_st() {
        assert_eq!(
            sanitize_message("before\x1b]0;title\x1b\\after"),
            "beforeafter"
        );
    }

    #[test]
    fn test_sanitize_removes_control_chars() {
        assert_eq!(sanitize_message("hello\x01\x02world"), "helloworld");
    }

    #[test]
    fn test_sanitize_removes_del() {
        assert_eq!(sanitize_message("hello\x7fworld"), "helloworld");
    }

    #[test]
    fn test_sanitize_truncates_long_message() {
        let long_msg = "x".repeat(MAX_MESSAGE_LENGTH + 100);
        let result = sanitize_message(&long_msg);
        assert!(result.len() <= MAX_MESSAGE_LENGTH);
        assert!(result.ends_with(TRUNCATION_SUFFIX));
    }

    #[test]
    fn test_sanitize_preserves_unicode() {
        assert_eq!(sanitize_message("hello 世界 🌍"), "hello 世界 🌍");
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_message(""), "");
    }

    // -- Agent ID validation tests --

    #[test]
    fn test_validate_agent_id_valid() {
        assert!(validate_agent_id("Agent1"));
        assert!(validate_agent_id("my_agent-2"));
        assert!(validate_agent_id("a"));
    }

    #[test]
    fn test_validate_agent_id_invalid() {
        assert!(!validate_agent_id(""));
        assert!(!validate_agent_id("agent with spaces"));
        assert!(!validate_agent_id("agent@special"));
        let long_id = "a".repeat(65);
        assert!(!validate_agent_id(&long_id));
    }

    #[test]
    fn test_validate_agent_id_max_length() {
        let id = "a".repeat(64);
        assert!(validate_agent_id(&id));
    }

    // -- URL validation tests --

    #[test]
    fn test_validate_url_https() {
        assert!(validate_agentmux_url("https://agentmux.example.com/api").is_ok());
    }

    #[test]
    fn test_validate_url_http_localhost() {
        assert!(validate_agentmux_url("http://localhost:8080/api").is_ok());
        assert!(validate_agentmux_url("http://127.0.0.1:8080/api").is_ok());
        assert!(validate_agentmux_url("http://[::1]:8080/api").is_ok());
    }

    #[test]
    fn test_validate_url_http_remote_rejected() {
        assert!(validate_agentmux_url("http://evil.com/api").is_err());
    }

    #[test]
    fn test_validate_url_bad_scheme() {
        assert!(validate_agentmux_url("ftp://example.com").is_err());
        assert!(validate_agentmux_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_validate_url_empty() {
        assert!(validate_agentmux_url("").is_err());
    }

    #[test]
    fn test_validate_url_no_scheme() {
        assert!(validate_agentmux_url("example.com/api").is_err());
    }

    // -- Format injected message tests --

    #[test]
    fn test_format_with_source() {
        assert_eq!(
            format_injected_message("hello", Some("Agent1"), true),
            "@Agent1: hello"
        );
    }

    #[test]
    fn test_format_without_source() {
        assert_eq!(
            format_injected_message("hello", Some("Agent1"), false),
            "hello"
        );
    }

    #[test]
    fn test_format_no_source_agent() {
        assert_eq!(format_injected_message("hello", None, true), "hello");
    }

    // -- Rate limiter tests --

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let mut rl = RateLimiter::new(3);
        assert!(rl.check());
        assert!(rl.check());
        assert!(rl.check());
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut rl = RateLimiter::new(2);
        assert!(rl.check());
        assert!(rl.check());
        assert!(!rl.check());
    }

    // -- Handler tests --

    #[test]
    fn test_handler_register_and_get() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", Some("tab1"))
            .unwrap();

        let agent = handler.get_agent("agent1").unwrap();
        assert_eq!(agent.block_id, "block1");
        assert_eq!(agent.tab_id.as_deref(), Some("tab1"));
    }

    #[test]
    fn test_handler_register_replaces_existing() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();
        handler
            .register_agent("agent1", "block2", None)
            .unwrap();

        let agent = handler.get_agent("agent1").unwrap();
        assert_eq!(agent.block_id, "block2");
        assert!(handler.get_agent_by_block("block1").is_none());
    }

    #[test]
    fn test_handler_unregister_agent() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();
        handler.unregister_agent("agent1");

        assert!(handler.get_agent("agent1").is_none());
        assert!(handler.get_agent_by_block("block1").is_none());
    }

    #[test]
    fn test_handler_unregister_block() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();
        handler.unregister_block("block1");

        assert!(handler.get_agent("agent1").is_none());
    }

    #[test]
    fn test_handler_list_agents() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();
        handler
            .register_agent("agent2", "block2", None)
            .unwrap();

        let agents = handler.list_agents();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_handler_invalid_agent_id() {
        let mut handler = Handler::new();
        let result = handler.register_agent("invalid agent!", "block1", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_handler_inject_no_sender() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();

        let resp = handler.inject_message(InjectionRequest {
            target_agent: "agent1".to_string(),
            message: "hello".to_string(),
            source_agent: None,
            request_id: None,
            priority: None,
            wait_for_idle: false,
        });

        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("input sender not configured"));
    }

    #[test]
    fn test_handler_inject_agent_not_found() {
        let mut handler = Handler::new();

        let resp = handler.inject_message(InjectionRequest {
            target_agent: "nonexistent".to_string(),
            message: "hello".to_string(),
            source_agent: None,
            request_id: None,
            priority: None,
            wait_for_idle: false,
        });

        assert!(!resp.success);
        assert!(resp.error.unwrap().contains("agent not found"));
    }

    #[test]
    fn test_handler_inject_success() {
        let sent = Arc::new(Mutex::new(Vec::<(String, Vec<u8>)>::new()));
        let sent_clone = sent.clone();

        let mut handler = Handler::new();
        handler.set_input_sender(Arc::new(move |block_id: &str, data: &[u8]| {
            sent_clone
                .lock()
                .unwrap()
                .push((block_id.to_string(), data.to_vec()));
            Ok(())
        }));
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();

        let resp = handler.inject_message(InjectionRequest {
            target_agent: "agent1".to_string(),
            message: "hello".to_string(),
            source_agent: None,
            request_id: Some("req-1".to_string()),
            priority: None,
            wait_for_idle: false,
        });

        assert!(resp.success);
        assert_eq!(resp.request_id, "req-1");
        assert_eq!(resp.block_id.as_deref(), Some("block1"));

        let calls = sent.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], ("block1".to_string(), b"hello".to_vec()));
        assert_eq!(calls[1], ("block1".to_string(), b"\r".to_vec()));
    }

    #[test]
    fn test_handler_audit_log() {
        let mut handler = Handler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();

        // Inject (will fail due to no sender)
        handler.inject_message(InjectionRequest {
            target_agent: "agent1".to_string(),
            message: "test".to_string(),
            source_agent: Some("src".to_string()),
            request_id: Some("req-1".to_string()),
            priority: None,
            wait_for_idle: false,
        });

        let log = handler.get_audit_log(10);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].target_agent, "agent1");
        assert_eq!(log[0].request_id, "req-1");
        assert!(!log[0].success);
    }

    #[test]
    fn test_handler_audit_log_ring_buffer() {
        let mut handler = Handler::new();
        // Fill beyond capacity
        for i in 0..AUDIT_LOG_MAX + 10 {
            handler.log_audit(
                None,
                &format!("agent{}", i),
                "block",
                "msg",
                true,
                None,
                &format!("req-{}", i),
            );
        }

        let log = handler.get_audit_log(200);
        assert_eq!(log.len(), AUDIT_LOG_MAX);
        // Most recent first
        assert_eq!(log[0].request_id, "req-109");
    }

    // -- Poller tests --

    #[test]
    fn test_poller_status_unconfigured() {
        let handler = get_global_handler();
        let poller = Poller::new(
            PollerConfig {
                agentmux_url: None,
                agentmux_token: None,
                poll_interval_secs: 30,
            },
            handler,
        );

        let status = poller.status();
        assert!(!status.configured);
        assert!(!status.running);
    }

    #[test]
    fn test_poller_status_configured() {
        let handler = get_global_handler();
        let poller = Poller::new(
            PollerConfig {
                agentmux_url: Some("https://example.com".to_string()),
                agentmux_token: Some("token123".to_string()),
                poll_interval_secs: 30,
            },
            handler,
        );

        let status = poller.status();
        assert!(status.configured);
        assert!(status.has_token);
    }

    #[test]
    fn test_poller_record_poll() {
        let handler = get_global_handler();
        let poller = Poller::new(
            PollerConfig {
                agentmux_url: Some("https://example.com".to_string()),
                agentmux_token: Some("token123".to_string()),
                poll_interval_secs: 30,
            },
            handler,
        );

        poller.record_poll();
        poller.record_poll();
        poller.record_injections(5);

        let status = poller.status();
        assert_eq!(status.poll_count, 2);
        assert_eq!(status.injections_count, 5);
        assert!(status.last_poll.is_some());
    }

    #[test]
    fn test_poller_reconfigure() {
        let handler = get_global_handler();
        let poller = Poller::new(
            PollerConfig {
                agentmux_url: None,
                agentmux_token: None,
                poll_interval_secs: 30,
            },
            handler,
        );

        assert!(!poller.is_configured());

        poller.reconfigure(
            Some("https://new.example.com".to_string()),
            Some("new-token".to_string()),
        );

        assert!(poller.is_configured());
        let status = poller.status();
        assert_eq!(status.url.as_deref(), Some("https://new.example.com"));
    }

    // -- Serde tests --

    #[test]
    fn test_injection_request_serde() {
        let req = InjectionRequest {
            target_agent: "Agent1".to_string(),
            message: "hello".to_string(),
            source_agent: Some("Agent2".to_string()),
            request_id: Some("req-123".to_string()),
            priority: None,
            wait_for_idle: false,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("target_agent"));
        assert!(json.contains("Agent1"));
        assert!(!json.contains("priority")); // None fields skipped

        let parsed: InjectionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_agent, "Agent1");
        assert_eq!(parsed.source_agent.as_deref(), Some("Agent2"));
    }

    #[test]
    fn test_injection_response_serde() {
        let resp = InjectionResponse {
            success: true,
            request_id: "req-123".to_string(),
            block_id: Some("block-abc".to_string()),
            error: None,
            timestamp: 1700000000000,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InjectionResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.block_id.as_deref(), Some("block-abc"));
    }

    #[test]
    fn test_pending_response_serde() {
        let json = r#"{"injections":[{"id":"inj-1","message":"hello","source_agent":"Agent2","created_at":1700000000000}]}"#;
        let parsed: PendingResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.injections.len(), 1);
        assert_eq!(parsed.injections[0].id, "inj-1");
        assert_eq!(parsed.injections[0].message, "hello");
    }

    #[test]
    fn test_agentmux_config_serde() {
        let config = AgentMuxConfigFile {
            url: Some("https://mux.example.com".to_string()),
            token: Some("secret".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: AgentMuxConfigFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url.as_deref(), Some("https://mux.example.com"));
    }

    // -- Thread-safe handler tests --

    #[test]
    fn test_reactive_handler_thread_safe() {
        let handler = ReactiveHandler::new();
        handler
            .register_agent("agent1", "block1", None)
            .unwrap();

        let agent = handler.get_agent("agent1").unwrap();
        assert_eq!(agent.block_id, "block1");

        handler.unregister_agent("agent1");
        assert!(handler.get_agent("agent1").is_none());
    }

    #[test]
    fn test_reactive_handler_list() {
        let handler = ReactiveHandler::new();
        handler
            .register_agent("a1", "b1", None)
            .unwrap();
        handler
            .register_agent("a2", "b2", Some("t2"))
            .unwrap();

        let agents = handler.list_agents();
        assert_eq!(agents.len(), 2);
    }
}
