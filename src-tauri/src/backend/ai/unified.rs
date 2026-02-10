// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Unified AI message types for the merged AI pane.
//!
//! These types normalize output from both chat backends (HTTP/SSE streaming)
//! and agent backends (subprocess NDJSON streams like Claude Code, Gemini CLI)
//! into a common format that the unified AI pane can render.
//!
//! The key abstraction is `UnifiedMessage` which contains `UnifiedMessagePart`
//! variants covering text, reasoning, tool use, tool results, errors, and more.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---- Backend type constants ----

/// Chat backend: HTTP/SSE streaming (Wave AI's existing providers).
pub const BACKEND_TYPE_CHAT: &str = "chat";

/// Agent backend: subprocess with NDJSON stream protocol (Claude Code, etc.).
pub const BACKEND_TYPE_AGENT: &str = "agent";

// ---- Agent backend identifiers ----

pub const AGENT_CLAUDE_CODE: &str = "claudecode";
pub const AGENT_GEMINI_CLI: &str = "gemini-cli";
pub const AGENT_CODEX_CLI: &str = "codex-cli";

// ---- Message roles ----

pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_TOOL: &str = "tool";

// ---- Unified message status ----

pub const MSG_STATUS_PENDING: &str = "pending";
pub const MSG_STATUS_STREAMING: &str = "streaming";
pub const MSG_STATUS_COMPLETE: &str = "complete";
pub const MSG_STATUS_ERROR: &str = "error";
pub const MSG_STATUS_CANCELLED: &str = "cancelled";

// ---- Tool approval status ----

pub const TOOL_APPROVAL_AUTO: &str = "auto";
pub const TOOL_APPROVAL_PENDING: &str = "pending";
pub const TOOL_APPROVAL_APPROVED: &str = "approved";
pub const TOOL_APPROVAL_DENIED: &str = "denied";

// ---- Agent backend configuration ----

/// Configuration for an agent backend (subprocess-based AI tool).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBackendConfig {
    /// Unique identifier (e.g., "claudecode", "gemini-cli").
    pub id: String,

    /// Human-readable display name (e.g., "Claude Code").
    pub display_name: String,

    /// Path to the executable binary.
    pub executable: String,

    /// Command-line arguments for NDJSON/streaming mode.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables to set for the subprocess.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Working directory for the subprocess.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Stream protocol: "ndjson" (Claude Code), "sse" (future), "raw" (fallback).
    #[serde(default = "default_stream_protocol")]
    pub stream_protocol: String,

    /// Whether this agent supports MCP server connection from the host app.
    #[serde(default)]
    pub supports_mcp: bool,

    /// Whether this agent supports pane-awareness tools.
    #[serde(default)]
    pub supports_pane_awareness: bool,

    /// Auto-detected from PATH if executable not absolute.
    #[serde(default)]
    pub auto_detected: bool,
}

fn default_stream_protocol() -> String {
    "ndjson".to_string()
}

impl Default for AgentBackendConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            executable: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            stream_protocol: "ndjson".to_string(),
            supports_mcp: false,
            supports_pane_awareness: false,
            auto_detected: false,
        }
    }
}

/// Pre-configured agent backend for Claude Code.
///
/// Uses `-p` (non-interactive) mode with full NDJSON streaming:
/// - `--output-format stream-json`: NDJSON output protocol
/// - `--verbose`: Include system events and tool details
/// - `--include-partial-messages`: Token-level streaming events
/// - `--input-format stream-json`: Accept structured follow-up messages via stdin
pub fn claude_code_config() -> AgentBackendConfig {
    let mut env = HashMap::new();
    // Disable non-essential traffic (autoupdater, telemetry, error reporting)
    env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        "1".to_string(),
    );
    // Prevent terminal title changes (we manage our own)
    env.insert(
        "CLAUDE_CODE_DISABLE_TERMINAL_TITLE".to_string(),
        "1".to_string(),
    );
    // Increase memory limit to prevent OOM on long sessions
    env.insert(
        "NODE_OPTIONS".to_string(),
        "--max-old-space-size=4096".to_string(),
    );

    AgentBackendConfig {
        id: AGENT_CLAUDE_CODE.to_string(),
        display_name: "Claude Code".to_string(),
        executable: "claude".to_string(),
        args: vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            "--include-partial-messages".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
        ],
        env,
        stream_protocol: "ndjson".to_string(),
        supports_mcp: true,
        supports_pane_awareness: true,
        ..Default::default()
    }
}

/// Pre-configured agent backend for Gemini CLI.
pub fn gemini_cli_config() -> AgentBackendConfig {
    AgentBackendConfig {
        id: AGENT_GEMINI_CLI.to_string(),
        display_name: "Gemini CLI".to_string(),
        executable: "gemini".to_string(),
        args: vec!["--output-format".to_string(), "json".to_string()],
        stream_protocol: "ndjson".to_string(),
        supports_mcp: false,
        supports_pane_awareness: false,
        ..Default::default()
    }
}

/// Pre-configured agent backend for Codex CLI.
pub fn codex_cli_config() -> AgentBackendConfig {
    AgentBackendConfig {
        id: AGENT_CODEX_CLI.to_string(),
        display_name: "Codex CLI".to_string(),
        executable: "codex".to_string(),
        args: vec!["--output-format".to_string(), "json".to_string()],
        stream_protocol: "ndjson".to_string(),
        supports_mcp: false,
        supports_pane_awareness: false,
        ..Default::default()
    }
}

// ---- Unified message types ----

/// A unified message in the AI conversation.
///
/// Normalizes both chat backend messages (from Wave AI's HTTP/SSE providers)
/// and agent backend messages (from subprocess NDJSON streams) into a common
/// format that the unified AI pane can render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMessage {
    /// Unique message ID.
    pub id: String,

    /// Message role: "user", "assistant", "system", "tool".
    pub role: String,

    /// Backend type that produced this message: "chat" or "agent".
    pub backend_type: String,

    /// Specific backend/agent ID (e.g., "claudecode", "openai", "anthropic").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub backend_id: String,

    /// Message parts (text, reasoning, tool use, etc.).
    pub parts: Vec<UnifiedMessagePart>,

    /// Message status: "pending", "streaming", "complete", "error", "cancelled".
    pub status: String,

    /// Model that generated this response (if known).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Token usage for this message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,

    /// Unix timestamp (milliseconds) when message was created.
    pub timestamp: i64,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub cache_read_tokens: i64,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub cache_write_tokens: i64,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// A single part of a unified message.
///
/// Covers all content types from both chat and agent backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UnifiedMessagePart {
    /// Plain text content.
    #[serde(rename = "text")]
    Text { text: String },

    /// Reasoning/thinking content (Anthropic extended thinking, etc.).
    #[serde(rename = "reasoning")]
    Reasoning { text: String },

    /// Tool invocation by the AI.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique ID for this tool call.
        call_id: String,
        /// Tool name (e.g., "read_file", "Bash", "Edit").
        name: String,
        /// Tool input parameters.
        input: serde_json::Value,
        /// Human-readable one-liner summary.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        summary: String,
        /// Approval status: "auto", "pending", "approved", "denied".
        #[serde(default, skip_serializing_if = "String::is_empty")]
        approval: String,
    },

    /// Result from a tool execution.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// ID of the tool call this result corresponds to.
        call_id: String,
        /// Result content.
        content: String,
        /// Whether the tool execution errored.
        #[serde(default)]
        is_error: bool,
    },

    /// File attachment or reference.
    #[serde(rename = "file")]
    File {
        filename: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        mimetype: String,
        #[serde(default, skip_serializing_if = "is_zero_usize")]
        size: usize,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        url: String,
    },

    /// Code diff (from agent tool output).
    #[serde(rename = "diff")]
    Diff {
        /// File path the diff applies to.
        path: String,
        /// Unified diff content.
        content: String,
    },

    /// Server-sent event metadata (cost, latency, etc.).
    #[serde(rename = "metadata")]
    Metadata {
        data: serde_json::Value,
    },

    /// Error part.
    #[serde(rename = "error")]
    Error { message: String },
}

fn is_zero_usize(v: &usize) -> bool {
    *v == 0
}

impl UnifiedMessage {
    /// Create a new user message.
    pub fn user(id: String, text: String, backend_type: &str) -> Self {
        Self {
            id,
            role: ROLE_USER.to_string(),
            backend_type: backend_type.to_string(),
            backend_id: String::new(),
            parts: vec![UnifiedMessagePart::Text { text }],
            status: MSG_STATUS_COMPLETE.to_string(),
            model: None,
            usage: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create a new empty assistant message (for streaming).
    pub fn assistant_streaming(id: String, backend_type: &str, backend_id: &str) -> Self {
        Self {
            id,
            role: ROLE_ASSISTANT.to_string(),
            backend_type: backend_type.to_string(),
            backend_id: backend_id.to_string(),
            parts: Vec::new(),
            status: MSG_STATUS_STREAMING.to_string(),
            model: None,
            usage: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create an error message.
    pub fn error(id: String, message: String, backend_type: &str) -> Self {
        Self {
            id,
            role: ROLE_ASSISTANT.to_string(),
            backend_type: backend_type.to_string(),
            backend_id: String::new(),
            parts: vec![UnifiedMessagePart::Error { message }],
            status: MSG_STATUS_ERROR.to_string(),
            model: None,
            usage: None,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Mark message as complete.
    pub fn set_complete(&mut self) {
        self.status = MSG_STATUS_COMPLETE.to_string();
    }

    /// Mark message as cancelled.
    pub fn set_cancelled(&mut self) {
        self.status = MSG_STATUS_CANCELLED.to_string();
    }

    /// Append a text part, merging with the last text part if possible.
    pub fn append_text(&mut self, text: &str) {
        if let Some(UnifiedMessagePart::Text { text: existing }) = self.parts.last_mut() {
            existing.push_str(text);
        } else {
            self.parts.push(UnifiedMessagePart::Text {
                text: text.to_string(),
            });
        }
    }

    /// Append a reasoning part, merging with the last reasoning part if possible.
    pub fn append_reasoning(&mut self, text: &str) {
        if let Some(UnifiedMessagePart::Reasoning { text: existing }) = self.parts.last_mut() {
            existing.push_str(text);
        } else {
            self.parts.push(UnifiedMessagePart::Reasoning {
                text: text.to_string(),
            });
        }
    }

    /// Add a tool use part.
    pub fn add_tool_use(
        &mut self,
        call_id: String,
        name: String,
        input: serde_json::Value,
        summary: String,
    ) {
        self.parts.push(UnifiedMessagePart::ToolUse {
            call_id,
            name,
            input,
            summary,
            approval: String::new(),
        });
    }

    /// Add a tool result part.
    pub fn add_tool_result(&mut self, call_id: String, content: String, is_error: bool) {
        self.parts.push(UnifiedMessagePart::ToolResult {
            call_id,
            content,
            is_error,
        });
    }

    /// Get all text parts concatenated.
    pub fn full_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                UnifiedMessagePart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Check if this message has any tool use parts.
    pub fn has_tool_use(&self) -> bool {
        self.parts
            .iter()
            .any(|p| matches!(p, UnifiedMessagePart::ToolUse { .. }))
    }
}

// ---- Unified conversation ----

/// A full conversation in the unified AI pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedConversation {
    /// Unique conversation ID.
    pub id: String,

    /// Backend type: "chat" or "agent".
    pub backend_type: String,

    /// Specific backend ID (e.g., "claudecode", "openai").
    pub backend_id: String,

    /// Ordered list of messages.
    pub messages: Vec<UnifiedMessage>,

    /// Current model being used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Cumulative token usage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_usage: Option<TokenUsage>,
}

impl UnifiedConversation {
    pub fn new(id: String, backend_type: &str, backend_id: &str) -> Self {
        Self {
            id,
            backend_type: backend_type.to_string(),
            backend_id: backend_id.to_string(),
            messages: Vec::new(),
            model: None,
            total_usage: None,
        }
    }

    pub fn add_message(&mut self, msg: UnifiedMessage) {
        self.messages.push(msg);
    }

    pub fn last_message_mut(&mut self) -> Option<&mut UnifiedMessage> {
        self.messages.last_mut()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_message_user() {
        let msg = UnifiedMessage::user("msg-1".into(), "Hello AI".into(), BACKEND_TYPE_CHAT);
        assert_eq!(msg.role, ROLE_USER);
        assert_eq!(msg.backend_type, BACKEND_TYPE_CHAT);
        assert_eq!(msg.status, MSG_STATUS_COMPLETE);
        assert_eq!(msg.full_text(), "Hello AI");
        assert!(!msg.has_tool_use());
    }

    #[test]
    fn test_unified_message_streaming() {
        let mut msg = UnifiedMessage::assistant_streaming(
            "msg-2".into(),
            BACKEND_TYPE_AGENT,
            AGENT_CLAUDE_CODE,
        );
        assert_eq!(msg.status, MSG_STATUS_STREAMING);
        assert_eq!(msg.backend_id, AGENT_CLAUDE_CODE);
        assert!(msg.parts.is_empty());

        msg.append_text("Hello ");
        msg.append_text("world");
        assert_eq!(msg.full_text(), "Hello world");
        assert_eq!(msg.parts.len(), 1); // Merged into one text part

        msg.set_complete();
        assert_eq!(msg.status, MSG_STATUS_COMPLETE);
    }

    #[test]
    fn test_unified_message_reasoning() {
        let mut msg = UnifiedMessage::assistant_streaming(
            "msg-3".into(),
            BACKEND_TYPE_CHAT,
            "anthropic",
        );
        msg.append_reasoning("Let me think...");
        msg.append_reasoning(" about this.");
        msg.append_text("Here is my answer.");

        assert_eq!(msg.parts.len(), 2); // reasoning + text
        match &msg.parts[0] {
            UnifiedMessagePart::Reasoning { text } => {
                assert_eq!(text, "Let me think... about this.");
            }
            _ => panic!("Expected reasoning part"),
        }
    }

    #[test]
    fn test_unified_message_tool_use() {
        let mut msg = UnifiedMessage::assistant_streaming(
            "msg-4".into(),
            BACKEND_TYPE_AGENT,
            AGENT_CLAUDE_CODE,
        );
        msg.append_text("Let me read that file.");
        msg.add_tool_use(
            "call-1".into(),
            "read_file".into(),
            serde_json::json!({"path": "/tmp/test.txt"}),
            "Read /tmp/test.txt".into(),
        );
        msg.add_tool_result("call-1".into(), "file contents here".into(), false);

        assert!(msg.has_tool_use());
        assert_eq!(msg.parts.len(), 3);

        match &msg.parts[1] {
            UnifiedMessagePart::ToolUse {
                call_id,
                name,
                summary,
                ..
            } => {
                assert_eq!(call_id, "call-1");
                assert_eq!(name, "read_file");
                assert_eq!(summary, "Read /tmp/test.txt");
            }
            _ => panic!("Expected tool use part"),
        }

        match &msg.parts[2] {
            UnifiedMessagePart::ToolResult {
                call_id,
                content,
                is_error,
            } => {
                assert_eq!(call_id, "call-1");
                assert_eq!(content, "file contents here");
                assert!(!is_error);
            }
            _ => panic!("Expected tool result part"),
        }
    }

    #[test]
    fn test_unified_message_error() {
        let msg = UnifiedMessage::error("msg-5".into(), "API timeout".into(), BACKEND_TYPE_CHAT);
        assert_eq!(msg.status, MSG_STATUS_ERROR);
        match &msg.parts[0] {
            UnifiedMessagePart::Error { message } => assert_eq!(message, "API timeout"),
            _ => panic!("Expected error part"),
        }
    }

    #[test]
    fn test_unified_message_serde_roundtrip() {
        let mut msg = UnifiedMessage::assistant_streaming(
            "msg-6".into(),
            BACKEND_TYPE_AGENT,
            AGENT_CLAUDE_CODE,
        );
        msg.model = Some("claude-sonnet-4-5".into());
        msg.usage = Some(TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        });
        msg.append_text("Hello");
        msg.add_tool_use(
            "c1".into(),
            "Bash".into(),
            serde_json::json!({"command": "ls"}),
            "Run ls".into(),
        );
        msg.set_complete();

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: UnifiedMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "msg-6");
        assert_eq!(parsed.role, ROLE_ASSISTANT);
        assert_eq!(parsed.backend_type, BACKEND_TYPE_AGENT);
        assert_eq!(parsed.backend_id, AGENT_CLAUDE_CODE);
        assert_eq!(parsed.status, MSG_STATUS_COMPLETE);
        assert_eq!(parsed.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(parsed.parts.len(), 2);
        assert_eq!(parsed.usage.as_ref().unwrap().input_tokens, 100);
    }

    #[test]
    fn test_unified_message_part_serde() {
        let parts: Vec<UnifiedMessagePart> = vec![
            UnifiedMessagePart::Text {
                text: "hello".into(),
            },
            UnifiedMessagePart::Reasoning {
                text: "thinking".into(),
            },
            UnifiedMessagePart::ToolUse {
                call_id: "c1".into(),
                name: "read_file".into(),
                input: serde_json::json!({"path": "/tmp"}),
                summary: String::new(),
                approval: TOOL_APPROVAL_AUTO.into(),
            },
            UnifiedMessagePart::ToolResult {
                call_id: "c1".into(),
                content: "ok".into(),
                is_error: false,
            },
            UnifiedMessagePart::File {
                filename: "test.txt".into(),
                mimetype: "text/plain".into(),
                size: 42,
                url: String::new(),
            },
            UnifiedMessagePart::Diff {
                path: "src/main.rs".into(),
                content: "+new line".into(),
            },
            UnifiedMessagePart::Error {
                message: "oops".into(),
            },
        ];

        let json = serde_json::to_string(&parts).unwrap();
        let parsed: Vec<UnifiedMessagePart> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 7);

        // Verify discriminant tags
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"type\":\"reasoning\""));
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"type\":\"tool_result\""));
        assert!(json.contains("\"type\":\"file\""));
        assert!(json.contains("\"type\":\"diff\""));
        assert!(json.contains("\"type\":\"error\""));
    }

    #[test]
    fn test_unified_conversation() {
        let mut conv =
            UnifiedConversation::new("conv-1".into(), BACKEND_TYPE_AGENT, AGENT_CLAUDE_CODE);
        assert_eq!(conv.message_count(), 0);

        conv.add_message(UnifiedMessage::user(
            "m1".into(),
            "hi".into(),
            BACKEND_TYPE_AGENT,
        ));
        assert_eq!(conv.message_count(), 1);

        let last = conv.last_message_mut().unwrap();
        assert_eq!(last.role, ROLE_USER);
    }

    #[test]
    fn test_agent_backend_config_serde() {
        let config = claude_code_config();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"id\":\"claudecode\""));
        assert!(json.contains("\"stream-json\""));
        assert!(json.contains("\"supports_mcp\":true"));

        let parsed: AgentBackendConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, AGENT_CLAUDE_CODE);
        assert_eq!(parsed.display_name, "Claude Code");
        assert!(parsed.supports_mcp);
        assert!(parsed.supports_pane_awareness);
    }

    #[test]
    fn test_agent_backend_config_defaults() {
        let config: AgentBackendConfig = serde_json::from_str(r#"{"id":"test","display_name":"Test","executable":"test-bin"}"#).unwrap();
        assert_eq!(config.stream_protocol, "ndjson");
        assert!(!config.supports_mcp);
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.cwd.is_none());
    }

    #[test]
    fn test_gemini_cli_config() {
        let config = gemini_cli_config();
        assert_eq!(config.id, AGENT_GEMINI_CLI);
        assert_eq!(config.display_name, "Gemini CLI");
        assert!(!config.supports_mcp);
    }

    #[test]
    fn test_token_usage_serde() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
        };
        let json = serde_json::to_string(&usage).unwrap();
        // Zero cache tokens should be omitted
        assert!(!json.contains("cache_read"));
        assert!(!json.contains("cache_write"));

        let usage2 = TokenUsage {
            cache_read_tokens: 200,
            ..usage
        };
        let json2 = serde_json::to_string(&usage2).unwrap();
        assert!(json2.contains("\"cache_read_tokens\":200"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(BACKEND_TYPE_CHAT, "chat");
        assert_eq!(BACKEND_TYPE_AGENT, "agent");
        assert_eq!(ROLE_USER, "user");
        assert_eq!(ROLE_ASSISTANT, "assistant");
        assert_eq!(MSG_STATUS_STREAMING, "streaming");
        assert_eq!(TOOL_APPROVAL_PENDING, "pending");
    }

    #[test]
    fn test_unified_message_cancelled() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_AGENT, "claudecode");
        msg.append_text("partial response");
        msg.set_cancelled();
        assert_eq!(msg.status, MSG_STATUS_CANCELLED);
        assert_eq!(msg.full_text(), "partial response");
    }

    #[test]
    fn test_text_after_tool_creates_new_part() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_AGENT, "claudecode");
        msg.append_text("Before tool.");
        msg.add_tool_use(
            "c1".into(),
            "Bash".into(),
            serde_json::json!({}),
            String::new(),
        );
        msg.append_text("After tool.");

        // Should be 3 parts: text, tool_use, text (not merged)
        assert_eq!(msg.parts.len(), 3);
        match &msg.parts[2] {
            UnifiedMessagePart::Text { text } => assert_eq!(text, "After tool."),
            _ => panic!("Expected text part"),
        }
    }
}
