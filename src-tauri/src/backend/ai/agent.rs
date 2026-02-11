// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Agent controller: manages lifecycle of agent backend subprocesses.
//!
//! An agent backend is an external AI tool (Claude Code, Gemini CLI, Codex CLI)
//! that runs as a subprocess and communicates via NDJSON or similar stream
//! protocol. The `AgentController` manages spawning, input/output routing,
//! interruption, and cleanup of these subprocesses.
//!
//! This module provides types and logic but does NOT directly spawn processes
//! (that requires Tauri's process APIs). Instead, it provides the state machine
//! and configuration, while the actual Tauri command layer handles spawning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

use super::unified::{AgentBackendConfig, UnifiedConversation, UnifiedMessage};

// ---- Agent status ----

/// Lifecycle status of an agent subprocess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum AgentStatus {
    /// Controller created, subprocess not yet started.
    #[serde(rename = "init")]
    Init,

    /// Subprocess is starting (binary found, spawn in progress).
    #[serde(rename = "starting")]
    Starting,

    /// Subprocess is running and ready for input.
    #[serde(rename = "running")]
    Running,

    /// Subprocess is processing a request (streaming response).
    #[serde(rename = "busy")]
    Busy,

    /// Subprocess exited normally.
    #[serde(rename = "done")]
    Done {
        #[serde(default)]
        exit_code: i32,
    },

    /// Subprocess failed or crashed.
    #[serde(rename = "error")]
    Error { message: String },
}

impl AgentStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentStatus::Done { .. } | AgentStatus::Error { .. })
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self,
            AgentStatus::Running | AgentStatus::Busy | AgentStatus::Starting
        )
    }
}

impl Default for AgentStatus {
    fn default() -> Self {
        AgentStatus::Init
    }
}

// ---- Agent controller state ----

/// State for a single agent subprocess instance.
///
/// This is the Rust-side state machine for an agent. The actual subprocess
/// handle lives in the Tauri layer (not here), but this tracks the logical
/// state, configuration, conversation, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Unique ID for this agent instance.
    pub instance_id: String,

    /// The pane/block ID this agent is associated with.
    pub pane_id: String,

    /// Agent backend configuration.
    pub config: AgentBackendConfig,

    /// Current status.
    pub status: AgentStatus,

    /// The conversation managed by this agent.
    pub conversation: UnifiedConversation,

    /// Session metadata from the agent (model name, session ID, etc.).
    #[serde(default)]
    pub session_meta: HashMap<String, String>,
}

impl AgentState {
    pub fn new(instance_id: String, pane_id: String, config: AgentBackendConfig) -> Self {
        let backend_id = config.id.clone();
        Self {
            instance_id: instance_id.clone(),
            pane_id,
            config,
            status: AgentStatus::Init,
            conversation: UnifiedConversation::new(
                instance_id,
                super::unified::BACKEND_TYPE_AGENT,
                &backend_id,
            ),
            session_meta: HashMap::new(),
        }
    }

    /// Transition to Starting status.
    pub fn set_starting(&mut self) {
        self.status = AgentStatus::Starting;
    }

    /// Transition to Running status.
    pub fn set_running(&mut self) {
        self.status = AgentStatus::Running;
    }

    /// Transition to Busy status (processing a request).
    pub fn set_busy(&mut self) {
        self.status = AgentStatus::Busy;
    }

    /// Transition to Done status.
    pub fn set_done(&mut self, exit_code: i32) {
        self.status = AgentStatus::Done { exit_code };
    }

    /// Transition to Error status.
    pub fn set_error(&mut self, message: String) {
        self.status = AgentStatus::Error { message };
    }

    /// Add a message to the conversation.
    pub fn add_message(&mut self, msg: UnifiedMessage) {
        self.conversation.add_message(msg);
    }

    /// Get the last assistant message for streaming updates.
    pub fn last_message_mut(&mut self) -> Option<&mut UnifiedMessage> {
        self.conversation.last_message_mut()
    }

    /// Check if the agent can accept new input.
    pub fn can_accept_input(&self) -> bool {
        matches!(self.status, AgentStatus::Running)
    }
}

// ---- Agent registry ----

/// Registry of all active agent instances.
///
/// Thread-safe registry that maps pane IDs to their agent state.
/// In the Tauri app, this would be stored in AppState.
pub struct AgentRegistry {
    agents: Mutex<HashMap<String, AgentState>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new agent for a pane.
    pub fn register(&self, state: AgentState) -> Result<(), String> {
        let mut agents = self.agents.lock().unwrap();
        let pane_id = state.pane_id.clone();
        if agents.contains_key(&pane_id) {
            return Err(format!("agent already registered for pane {pane_id}"));
        }
        agents.insert(pane_id, state);
        Ok(())
    }

    /// Get a clone of the agent state for a pane.
    pub fn get(&self, pane_id: &str) -> Option<AgentState> {
        self.agents.lock().unwrap().get(pane_id).cloned()
    }

    /// Update agent state via a closure.
    pub fn update<F, R>(&self, pane_id: &str, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut AgentState) -> R,
    {
        let mut agents = self.agents.lock().unwrap();
        let state = agents
            .get_mut(pane_id)
            .ok_or_else(|| format!("no agent for pane {pane_id}"))?;
        Ok(f(state))
    }

    /// Remove an agent from the registry.
    pub fn remove(&self, pane_id: &str) -> Option<AgentState> {
        self.agents.lock().unwrap().remove(pane_id)
    }

    /// Get all pane IDs with active (non-terminal) agents.
    pub fn active_pane_ids(&self) -> Vec<String> {
        self.agents
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, s)| !s.status.is_terminal())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get count of registered agents.
    pub fn len(&self) -> usize {
        self.agents.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.lock().unwrap().is_empty()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Spawn request/response types ----

/// Request to spawn a new agent subprocess.
/// Sent from frontend to Tauri backend via IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentRequest {
    /// Pane/block ID to associate with.
    pub pane_id: String,

    /// Agent backend ID (e.g., "claudecode").
    pub backend_id: String,

    /// Optional working directory override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Optional extra environment variables.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,

    /// Initial prompt to send after spawn (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,

    /// Resume a previous Claude Code session by ID (optional).
    /// When provided, spawns with `--resume {session_id}` flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_session_id: Option<String>,
}

/// Response after spawning an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnAgentResponse {
    /// Unique instance ID for this agent.
    pub instance_id: String,

    /// Current status after spawn.
    pub status: AgentStatus,
}

/// Request to send input to a running agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInputRequest {
    /// Pane ID of the target agent.
    pub pane_id: String,

    /// Text input to send.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Signal to send (e.g., "SIGINT").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
}

/// Status update event emitted from agent to frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEvent {
    /// Pane ID of the agent.
    pub pane_id: String,

    /// Instance ID of the agent.
    pub instance_id: String,

    /// New status.
    pub status: AgentStatus,

    /// Optional error message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::ai::unified::{claude_code_config, BACKEND_TYPE_AGENT};

    #[test]
    fn test_agent_status_default() {
        let status = AgentStatus::default();
        assert_eq!(status, AgentStatus::Init);
        assert!(!status.is_terminal());
        assert!(!status.is_running());
    }

    #[test]
    fn test_agent_status_transitions() {
        assert!(!AgentStatus::Init.is_running());
        assert!(AgentStatus::Starting.is_running());
        assert!(AgentStatus::Running.is_running());
        assert!(AgentStatus::Busy.is_running());
        assert!(!AgentStatus::Done { exit_code: 0 }.is_running());
        assert!(!AgentStatus::Error {
            message: "x".into()
        }
        .is_running());

        assert!(AgentStatus::Done { exit_code: 0 }.is_terminal());
        assert!(AgentStatus::Error {
            message: "x".into()
        }
        .is_terminal());
        assert!(!AgentStatus::Running.is_terminal());
    }

    #[test]
    fn test_agent_status_serde() {
        let statuses = vec![
            AgentStatus::Init,
            AgentStatus::Starting,
            AgentStatus::Running,
            AgentStatus::Busy,
            AgentStatus::Done { exit_code: 0 },
            AgentStatus::Error {
                message: "fail".into(),
            },
        ];

        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let parsed: AgentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(&parsed, status);
        }

        let json = serde_json::to_string(&AgentStatus::Done { exit_code: 1 }).unwrap();
        assert!(json.contains("\"status\":\"done\""));
        assert!(json.contains("\"exit_code\":1"));
    }

    #[test]
    fn test_agent_state_new() {
        let config = claude_code_config();
        let state = AgentState::new("inst-1".into(), "pane-1".into(), config);

        assert_eq!(state.instance_id, "inst-1");
        assert_eq!(state.pane_id, "pane-1");
        assert_eq!(state.status, AgentStatus::Init);
        assert_eq!(state.config.id, "claudecode");
        assert_eq!(state.conversation.backend_type, BACKEND_TYPE_AGENT);
        assert_eq!(state.conversation.message_count(), 0);
    }

    #[test]
    fn test_agent_state_lifecycle() {
        let config = claude_code_config();
        let mut state = AgentState::new("inst-1".into(), "pane-1".into(), config);

        assert!(!state.can_accept_input()); // Init

        state.set_starting();
        assert!(!state.can_accept_input()); // Starting

        state.set_running();
        assert!(state.can_accept_input()); // Running

        state.set_busy();
        assert!(!state.can_accept_input()); // Busy

        state.set_running();
        assert!(state.can_accept_input()); // Back to Running

        state.set_done(0);
        assert!(!state.can_accept_input()); // Done
        assert!(state.status.is_terminal());
    }

    #[test]
    fn test_agent_state_conversation() {
        let config = claude_code_config();
        let mut state = AgentState::new("inst-1".into(), "pane-1".into(), config);

        let msg = UnifiedMessage::user("m1".into(), "hello".into(), BACKEND_TYPE_AGENT);
        state.add_message(msg);
        assert_eq!(state.conversation.message_count(), 1);

        let last = state.last_message_mut().unwrap();
        assert_eq!(last.full_text(), "hello");
    }

    #[test]
    fn test_agent_registry_basic() {
        let registry = AgentRegistry::new();
        assert!(registry.is_empty());

        let config = claude_code_config();
        let state = AgentState::new("inst-1".into(), "pane-1".into(), config);
        registry.register(state).unwrap();

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let got = registry.get("pane-1").unwrap();
        assert_eq!(got.instance_id, "inst-1");
    }

    #[test]
    fn test_agent_registry_duplicate() {
        let registry = AgentRegistry::new();
        let config = claude_code_config();

        let state1 = AgentState::new("inst-1".into(), "pane-1".into(), config.clone());
        let state2 = AgentState::new("inst-2".into(), "pane-1".into(), config);

        registry.register(state1).unwrap();
        let result = registry.register(state2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already registered"));
    }

    #[test]
    fn test_agent_registry_update() {
        let registry = AgentRegistry::new();
        let config = claude_code_config();
        let state = AgentState::new("inst-1".into(), "pane-1".into(), config);
        registry.register(state).unwrap();

        registry
            .update("pane-1", |s| {
                s.set_running();
            })
            .unwrap();

        let got = registry.get("pane-1").unwrap();
        assert_eq!(got.status, AgentStatus::Running);
    }

    #[test]
    fn test_agent_registry_update_nonexistent() {
        let registry = AgentRegistry::new();
        let result = registry.update("nonexistent", |_| {});
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_registry_remove() {
        let registry = AgentRegistry::new();
        let config = claude_code_config();
        let state = AgentState::new("inst-1".into(), "pane-1".into(), config);
        registry.register(state).unwrap();

        let removed = registry.remove("pane-1");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().instance_id, "inst-1");
        assert!(registry.is_empty());
    }

    #[test]
    fn test_agent_registry_active_pane_ids() {
        let registry = AgentRegistry::new();
        let config = claude_code_config();

        let mut state1 = AgentState::new("inst-1".into(), "pane-1".into(), config.clone());
        state1.set_running();
        registry.register(state1).unwrap();

        let mut state2 = AgentState::new("inst-2".into(), "pane-2".into(), config.clone());
        state2.set_done(0);
        registry.register(state2).unwrap();

        let mut state3 = AgentState::new("inst-3".into(), "pane-3".into(), config);
        state3.set_busy();
        registry.register(state3).unwrap();

        let active = registry.active_pane_ids();
        assert_eq!(active.len(), 2);
        assert!(active.contains(&"pane-1".to_string()));
        assert!(active.contains(&"pane-3".to_string()));
    }

    #[test]
    fn test_spawn_agent_request_serde() {
        let req = SpawnAgentRequest {
            pane_id: "pane-1".into(),
            backend_id: "claudecode".into(),
            cwd: Some("/home/user/project".into()),
            env: HashMap::new(),
            initial_prompt: Some("explain this codebase".into()),
            resume_session_id: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: SpawnAgentRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pane_id, "pane-1");
        assert_eq!(parsed.backend_id, "claudecode");
        assert_eq!(parsed.cwd.as_deref(), Some("/home/user/project"));
        assert_eq!(
            parsed.initial_prompt.as_deref(),
            Some("explain this codebase")
        );
    }

    #[test]
    fn test_spawn_agent_response_serde() {
        let resp = SpawnAgentResponse {
            instance_id: "inst-1".into(),
            status: AgentStatus::Starting,
        };

        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SpawnAgentResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.instance_id, "inst-1");
        assert_eq!(parsed.status, AgentStatus::Starting);
    }

    #[test]
    fn test_agent_input_request_serde() {
        let req = AgentInputRequest {
            pane_id: "pane-1".into(),
            text: Some("hello".into()),
            signal: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("\"signal\"")); // None should be omitted

        let sig_req = AgentInputRequest {
            pane_id: "pane-1".into(),
            text: None,
            signal: Some("SIGINT".into()),
        };
        let json2 = serde_json::to_string(&sig_req).unwrap();
        assert!(json2.contains("\"SIGINT\""));
        assert!(!json2.contains("\"text\"")); // None omitted
    }

    #[test]
    fn test_agent_status_event_serde() {
        let event = AgentStatusEvent {
            pane_id: "pane-1".into(),
            instance_id: "inst-1".into(),
            status: AgentStatus::Error {
                message: "binary not found".into(),
            },
            error: Some("binary not found".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"pane_id\":\"pane-1\""));
        assert!(json.contains("\"status\":\"error\""));
    }

    #[test]
    fn test_agent_registry_thread_safety() {
        use std::thread;

        let registry = Arc::new(AgentRegistry::new());
        let mut handles = vec![];

        for i in 0..10 {
            let reg = registry.clone();
            handles.push(thread::spawn(move || {
                let config = claude_code_config();
                let state =
                    AgentState::new(format!("inst-{i}"), format!("pane-{i}"), config);
                reg.register(state).unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(registry.len(), 10);
    }
}
