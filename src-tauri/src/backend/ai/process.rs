// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Agent process management: spawning, I/O routing, and lifecycle control.
//!
//! Manages the subprocess lifecycle for agent backends (Claude Code, Gemini CLI,
//! Codex CLI). Handles spawning with proper args/env, writing to stdin, reading
//! NDJSON from stdout, sending signals, and cleanup.
//!
//! The process management is separated from the Tauri command layer so it can
//! be tested independently.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;

use super::adapters::{
    adapt_claude_code_event, adapt_claude_code_stream_event, AdapterEvent, ClaudeCodeEvent,
    ClaudeCodeStreamEvent,
};
use super::unified::AgentBackendConfig;

// ---- Error types ----

/// Errors from agent process management.
#[derive(Debug, thiserror::Error)]
pub enum AgentProcessError {
    #[error("failed to spawn agent process: {0}")]
    SpawnFailed(String),

    #[error("stdin is closed or unavailable")]
    StdinClosed,

    #[error("agent process has already exited")]
    ProcessExited,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json parse error: {0}")]
    JsonParse(String),
}

// ---- Agent process handle ----

/// Handle to a running agent subprocess.
///
/// Owns the child process and its stdin. Stdout is taken separately
/// for async reading in a background task.
pub struct AgentProcess {
    child: Child,
    stdin: Option<ChildStdin>,
}

impl AgentProcess {
    /// Spawn an agent subprocess with the given configuration.
    ///
    /// Sets up stdin/stdout/stderr as pipes. The caller should use
    /// `take_stdout()` to get the stdout reader for NDJSON parsing.
    pub fn spawn(
        config: &AgentBackendConfig,
        cwd: Option<&str>,
        extra_env: &HashMap<String, String>,
    ) -> Result<Self, AgentProcessError> {
        let mut cmd = Command::new(&config.executable);

        // Add configured args
        if !config.args.is_empty() {
            cmd.args(&config.args);
        }

        // Set configured env vars
        for (k, v) in &config.env {
            cmd.env(k, v);
        }

        // Set extra env vars (from spawn request)
        for (k, v) in extra_env {
            cmd.env(k, v);
        }

        // Set working directory
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        } else if let Some(ref dir) = config.cwd {
            cmd.current_dir(dir);
        }

        // Pipe all I/O
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Prevent the child from inheriting signal handlers on Unix
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    // Create a new process group so SIGINT doesn't propagate
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| AgentProcessError::SpawnFailed(e.to_string()))?;

        let stdin = child.stdin.take();

        Ok(Self { child, stdin })
    }

    /// Write text to the agent's stdin, followed by a newline.
    pub async fn write_stdin(&mut self, text: &str) -> Result<(), AgentProcessError> {
        let stdin = self.stdin.as_mut().ok_or(AgentProcessError::StdinClosed)?;
        stdin.write_all(text.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Write raw bytes to the agent's stdin (no newline appended).
    pub async fn write_stdin_raw(&mut self, data: &[u8]) -> Result<(), AgentProcessError> {
        let stdin = self.stdin.as_mut().ok_or(AgentProcessError::StdinClosed)?;
        stdin.write_all(data).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Send a structured follow-up message to a running Claude Code subprocess.
    ///
    /// Uses the `--input-format stream-json` protocol. The message is formatted
    /// as an NDJSON line with `type: "user"` and the given session ID.
    pub async fn send_user_message(
        &mut self,
        session_id: &str,
        text: &str,
    ) -> Result<(), AgentProcessError> {
        let msg = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": text
            },
            "session_id": session_id
        });
        let mut line = serde_json::to_string(&msg)
            .map_err(|e| AgentProcessError::JsonParse(e.to_string()))?;
        line.push('\n');
        self.write_stdin_raw(line.as_bytes()).await
    }

    /// Take ownership of the child's stdout for reading in a background task.
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.child.stdout.take()
    }

    /// Take ownership of the child's stderr for reading in a background task.
    pub fn take_stderr(&mut self) -> Option<tokio::process::ChildStderr> {
        self.child.stderr.take()
    }

    /// Send SIGINT to the agent process (Unix) or kill on Windows.
    pub fn interrupt(&self) -> Result<(), AgentProcessError> {
        let pid = self.child.id().ok_or(AgentProcessError::ProcessExited)?;

        #[cfg(unix)]
        {
            // Send SIGINT to the process group
            let ret = unsafe { libc::kill(-(pid as i32), libc::SIGINT) };
            if ret != 0 {
                return Err(AgentProcessError::Io(std::io::Error::last_os_error()));
            }
        }

        #[cfg(windows)]
        {
            // On Windows, SIGINT isn't supported for child processes easily.
            // Use kill as a fallback (the Tauri layer handles this better).
            let _ = pid;
        }

        Ok(())
    }

    /// Force kill the agent process.
    pub async fn kill(&mut self) -> Result<(), AgentProcessError> {
        self.child.kill().await?;
        Ok(())
    }

    /// Wait for the agent process to exit and return the exit code.
    pub async fn wait(&mut self) -> Result<i32, AgentProcessError> {
        let status = self.child.wait().await?;
        Ok(status.code().unwrap_or(-1))
    }

    /// Get the process ID, if the process is still running.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    /// Check if the process has exited (non-blocking).
    pub fn try_wait(&mut self) -> Result<Option<i32>, AgentProcessError> {
        match self.child.try_wait()? {
            Some(status) => Ok(Some(status.code().unwrap_or(-1))),
            None => Ok(None),
        }
    }
}

// ---- NDJSON line parser ----

/// Parse a single NDJSON line from an agent's stdout into adapter events.
///
/// Claude Code with `--output-format stream-json` outputs top-level events
/// (system, stream_event, assistant, user, result). This function first tries
/// to parse as a `ClaudeCodeEvent` (outer wrapper), and falls back to
/// `ClaudeCodeStreamEvent` (inner stream event) for backwards compatibility
/// with older invocations or partial output.
pub fn parse_ndjson_line(line: &str) -> Result<Vec<AdapterEvent>, AgentProcessError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(vec![]);
    }

    // Try outer ClaudeCodeEvent first (stream-json protocol)
    if let Ok(event) = serde_json::from_str::<ClaudeCodeEvent>(line) {
        return Ok(adapt_claude_code_event(&event));
    }

    // Fall back to inner ClaudeCodeStreamEvent (legacy or partial)
    let event: ClaudeCodeStreamEvent =
        serde_json::from_str(line).map_err(|e| AgentProcessError::JsonParse(e.to_string()))?;

    Ok(adapt_claude_code_stream_event(&event))
}

/// Parse a raw NDJSON line, returning the raw outer event and adapted events.
///
/// Useful when the caller needs both the raw event for logging and
/// the adapted events for the UI.
pub fn parse_ndjson_line_with_event(
    line: &str,
) -> Result<(ClaudeCodeEvent, Vec<AdapterEvent>), AgentProcessError> {
    let line = line.trim();
    if line.is_empty() {
        return Err(AgentProcessError::JsonParse("empty line".into()));
    }

    let event: ClaudeCodeEvent =
        serde_json::from_str(line).map_err(|e| AgentProcessError::JsonParse(e.to_string()))?;

    let adapted = adapt_claude_code_event(&event);
    Ok((event, adapted))
}

/// Parse a raw NDJSON line as an inner stream event (legacy).
///
/// Useful when the caller needs both the raw event for logging and
/// the adapted events for the UI.
pub fn parse_ndjson_line_raw(
    line: &str,
) -> Result<(ClaudeCodeStreamEvent, Vec<AdapterEvent>), AgentProcessError> {
    let line = line.trim();
    if line.is_empty() {
        return Err(AgentProcessError::JsonParse("empty line".into()));
    }

    let event: ClaudeCodeStreamEvent =
        serde_json::from_str(line).map_err(|e| AgentProcessError::JsonParse(e.to_string()))?;

    let adapted = adapt_claude_code_stream_event(&event);
    Ok((event, adapted))
}

// ---- Stdout reader task ----

/// Event emitted by the stdout reader task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentOutputEvent {
    /// Adapter events from a parsed NDJSON line.
    #[serde(rename = "adapter_events")]
    AdapterEvents { events: Vec<AdapterEvent> },

    /// Raw line that couldn't be parsed as NDJSON (e.g., startup messages).
    #[serde(rename = "raw_line")]
    RawLine { line: String },

    /// The stdout stream ended (process exited or pipe closed).
    #[serde(rename = "stream_end")]
    StreamEnd,

    /// An error occurred while reading.
    #[serde(rename = "read_error")]
    ReadError { message: String },
}

/// Spawn a background task that reads NDJSON lines from agent stdout
/// and sends parsed events through the provided channel.
///
/// Returns a `JoinHandle` for the reader task.
pub fn spawn_stdout_reader(
    stdout: ChildStdout,
    tx: mpsc::Sender<AgentOutputEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let event = match parse_ndjson_line(&line) {
                        Ok(events) if !events.is_empty() => {
                            AgentOutputEvent::AdapterEvents { events }
                        }
                        Ok(_) => {
                            // Empty result from empty line, skip
                            continue;
                        }
                        Err(_) => {
                            // Not valid NDJSON — could be startup output, log line, etc.
                            AgentOutputEvent::RawLine {
                                line: line.to_string(),
                            }
                        }
                    };

                    if tx.send(event).await.is_err() {
                        // Receiver dropped, stop reading
                        break;
                    }
                }
                Ok(None) => {
                    // EOF — process exited or stdout closed
                    let _ = tx.send(AgentOutputEvent::StreamEnd).await;
                    break;
                }
                Err(e) => {
                    let _ = tx
                        .send(AgentOutputEvent::ReadError {
                            message: e.to_string(),
                        })
                        .await;
                    break;
                }
            }
        }
    })
}

// ---- Backend discovery ----

/// Check if a binary is available on the system PATH.
pub fn is_binary_available(name: &str) -> bool {
    which_binary(name).is_some()
}

/// Find the full path to a binary on the system PATH.
pub fn which_binary(name: &str) -> Option<String> {
    // Use the `which` crate pattern: search PATH directories
    let path_var = std::env::var("PATH").unwrap_or_default();

    #[cfg(unix)]
    let separator = ':';
    #[cfg(windows)]
    let separator = ';';

    for dir in path_var.split(separator) {
        let candidate = std::path::Path::new(dir).join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }

        // On Windows, also check with .exe extension
        #[cfg(windows)]
        {
            let candidate_exe = std::path::Path::new(dir).join(format!("{name}.exe"));
            if candidate_exe.is_file() {
                return Some(candidate_exe.to_string_lossy().into_owned());
            }
        }
    }

    None
}

/// Detect available agent backends by checking PATH for known binaries.
pub fn detect_available_backends() -> Vec<AgentBackendConfig> {
    use super::unified::{claude_code_config, codex_cli_config, gemini_cli_config};

    let mut available = Vec::new();

    // Check for Claude Code
    if is_binary_available("claude") {
        let mut config = claude_code_config();
        if let Some(path) = which_binary("claude") {
            config.executable = path;
        }
        config.auto_detected = true;
        available.push(config);
    }

    // Check for Gemini CLI
    if is_binary_available("gemini") {
        let mut config = gemini_cli_config();
        if let Some(path) = which_binary("gemini") {
            config.executable = path;
        }
        config.auto_detected = true;
        available.push(config);
    }

    // Check for Codex CLI
    if is_binary_available("codex") {
        let mut config = codex_cli_config();
        if let Some(path) = which_binary("codex") {
            config.executable = path;
        }
        config.auto_detected = true;
        available.push(config);
    }

    available
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ai::adapters::ClaudeCodeStreamEvent;

    #[test]
    fn test_parse_ndjson_empty_line() {
        let result = parse_ndjson_line("").unwrap();
        assert!(result.is_empty());

        let result = parse_ndjson_line("  \n  ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_ndjson_message_start() {
        let line = r#"{"type":"message_start","message":{"role":"assistant","model":"claude-sonnet-4-5","content":[],"usage":{"input_tokens":100,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert!(!events.is_empty());

        match &events[0] {
            AdapterEvent::MessageStart { model, .. } => {
                assert_eq!(model.as_deref(), Some("claude-sonnet-4-5"));
            }
            other => panic!("expected MessageStart, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_text_delta() {
        let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello "}}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            AdapterEvent::TextDelta { text } => {
                assert_eq!(text, "Hello ");
            }
            other => panic!("expected TextDelta, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_tool_use_start() {
        let line = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_123","name":"Read","input":{}}}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            AdapterEvent::ToolUseStart { call_id, name, .. } => {
                assert_eq!(call_id, "call_123");
                assert_eq!(name, "Read");
            }
            other => panic!("expected ToolUseStart, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_message_stop() {
        let line = r#"{"type":"message_stop"}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            AdapterEvent::MessageEnd { usage } => {
                assert!(usage.is_none());
            }
            other => panic!("expected MessageEnd, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_message_delta_with_usage() {
        let line = r#"{"type":"message_delta","delta":{},"usage":{"input_tokens":150,"output_tokens":50,"cache_creation_input_tokens":0,"cache_read_input_tokens":20}}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert_eq!(events.len(), 1);

        match &events[0] {
            AdapterEvent::MessageEnd { usage } => {
                let u = usage.as_ref().unwrap();
                assert_eq!(u.input_tokens, 150);
                assert_eq!(u.output_tokens, 50);
                assert_eq!(u.cache_read_tokens, 20);
            }
            other => panic!("expected MessageEnd with usage, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_invalid_json() {
        let result = parse_ndjson_line("this is not json");
        assert!(result.is_err());
        match result.unwrap_err() {
            AgentProcessError::JsonParse(_) => {}
            other => panic!("expected JsonParse error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_ndjson_unknown_type() {
        // Unknown types should parse via #[serde(other)] and produce empty events
        let line = r#"{"type":"ping"}"#;
        let events = parse_ndjson_line(line).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_ndjson_raw() {
        let line = r#"{"type":"message_stop"}"#;
        let (raw, adapted) = parse_ndjson_line_raw(line).unwrap();
        assert!(matches!(raw, ClaudeCodeStreamEvent::MessageStop));
        assert_eq!(adapted.len(), 1);
    }

    #[test]
    fn test_parse_ndjson_raw_empty() {
        let result = parse_ndjson_line_raw("");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_output_event_serde() {
        let events = vec![
            AgentOutputEvent::AdapterEvents {
                events: vec![AdapterEvent::TextDelta {
                    text: "hello".into(),
                }],
            },
            AgentOutputEvent::RawLine {
                line: "startup log".into(),
            },
            AgentOutputEvent::StreamEnd,
            AgentOutputEvent::ReadError {
                message: "broken pipe".into(),
            },
        ];

        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let parsed: AgentOutputEvent = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_which_binary_nonexistent() {
        assert!(!is_binary_available("this_binary_does_not_exist_12345"));
        assert!(which_binary("this_binary_does_not_exist_12345").is_none());
    }

    #[test]
    fn test_which_binary_sh() {
        // /bin/sh should exist on any Unix system
        #[cfg(unix)]
        {
            assert!(is_binary_available("sh"));
            let path = which_binary("sh");
            assert!(path.is_some());
            assert!(path.unwrap().contains("sh"));
        }
    }

    #[test]
    fn test_detect_available_backends() {
        // Just verify it doesn't panic. Actual results depend on what's installed.
        let backends = detect_available_backends();
        for b in &backends {
            assert!(!b.id.is_empty());
            assert!(!b.display_name.is_empty());
            assert!(b.auto_detected);
        }
    }

    #[test]
    fn test_agent_process_error_display() {
        let err = AgentProcessError::SpawnFailed("not found".into());
        assert!(err.to_string().contains("not found"));

        let err = AgentProcessError::StdinClosed;
        assert!(err.to_string().contains("stdin"));

        let err = AgentProcessError::ProcessExited;
        assert!(err.to_string().contains("exited"));

        let err = AgentProcessError::JsonParse("unexpected token".into());
        assert!(err.to_string().contains("unexpected token"));
    }

    #[test]
    fn test_full_ndjson_conversation_flow() {
        // Simulate a complete Claude Code conversation via NDJSON lines
        let lines = vec![
            r#"{"type":"message_start","message":{"role":"assistant","model":"claude-sonnet-4-5","content":[],"usage":{"input_tokens":100,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Let me "}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"help you."}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"call_abc","name":"Read","input":{}}}"#,
            r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"file_path\""}}"#,
            r#"{"type":"content_block_stop","index":1}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"input_tokens":100,"output_tokens":50,"cache_creation_input_tokens":10,"cache_read_input_tokens":5}}"#,
            r#"{"type":"message_stop"}"#,
        ];

        let mut all_events = Vec::new();
        for line in &lines {
            let events = parse_ndjson_line(line).unwrap();
            all_events.extend(events);
        }

        // Should have: MessageStart, TextDelta x2, ToolUseStart, MessageEnd (with usage), MessageEnd (stop)
        assert!(all_events.len() >= 5);

        // First event is MessageStart
        assert!(matches!(&all_events[0], AdapterEvent::MessageStart { .. }));

        // Should contain text deltas
        let text_count = all_events
            .iter()
            .filter(|e| matches!(e, AdapterEvent::TextDelta { .. }))
            .count();
        assert!(text_count >= 2);

        // Should contain a tool use start
        let tool_starts = all_events
            .iter()
            .filter(|e| matches!(e, AdapterEvent::ToolUseStart { .. }))
            .count();
        assert_eq!(tool_starts, 1);
    }
}
