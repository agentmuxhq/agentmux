// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tauri IPC commands for agent backend management.
//!
//! These commands allow the frontend to spawn, control, and query agent
//! subprocesses (Claude Code, Gemini CLI, Codex CLI) via Tauri's invoke system.
//!
//! Architecture:
//! - `spawn_agent`: Creates subprocess, registers in AgentRegistry, starts stdout reader
//! - `send_agent_input`: Writes user text to agent stdin
//! - `interrupt_agent`: Sends SIGINT to pause the agent
//! - `kill_agent`: Force-kills the agent subprocess
//! - `get_agent_status`: Returns current agent status for a pane
//! - `list_agent_backends`: Returns available agent backends (auto-detected from PATH)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex as TokioMutex};

use crate::backend::ai::agent::{
    AgentInputRequest, AgentRegistry, AgentState, AgentStatus, AgentStatusEvent,
    SpawnAgentRequest, SpawnAgentResponse,
};
use crate::backend::ai::process::{
    detect_available_backends, AgentOutputEvent, AgentProcess,
};
use crate::backend::ai::unified::{
    claude_code_config, codex_cli_config, gemini_cli_config, AgentBackendConfig,
};

// ---- Agent process store ----

/// Holds the actual OS process handles, separate from AgentRegistry
/// (which stores serializable state). Keyed by pane_id.
///
/// Uses tokio::Mutex because process I/O operations are async.
pub struct AgentProcessStore {
    processes: TokioMutex<HashMap<String, AgentProcess>>,
}

impl AgentProcessStore {
    pub fn new() -> Self {
        Self {
            processes: TokioMutex::new(HashMap::new()),
        }
    }

    pub async fn insert(&self, pane_id: String, process: AgentProcess) {
        self.processes.lock().await.insert(pane_id, process);
    }

    pub async fn remove(&self, pane_id: &str) -> Option<AgentProcess> {
        self.processes.lock().await.remove(pane_id)
    }

    pub async fn has(&self, pane_id: &str) -> bool {
        self.processes.lock().await.contains_key(pane_id)
    }

    /// Write to stdin of a process by pane_id.
    pub async fn write_stdin(&self, pane_id: &str, text: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        let process = processes
            .get_mut(pane_id)
            .ok_or_else(|| format!("no agent process for pane {pane_id}"))?;
        process
            .write_stdin(text)
            .await
            .map_err(|e| format!("failed to write stdin: {e}"))
    }

    /// Send structured input to a Claude Code process (--input-format stream-json).
    ///
    /// This method is used for multi-turn conversations when a session_id is available.
    /// It formats the input as NDJSON with the proper structure for Claude Code.
    pub async fn send_structured_input(
        &self,
        pane_id: &str,
        session_id: &str,
        text: &str,
    ) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        let process = processes
            .get_mut(pane_id)
            .ok_or_else(|| format!("no agent process for pane {pane_id}"))?;
        process
            .send_user_message(session_id, text)
            .await
            .map_err(|e| format!("failed to send structured input: {e}"))
    }

    /// Send interrupt signal to a process by pane_id.
    pub async fn interrupt(&self, pane_id: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        let process = processes
            .get_mut(pane_id)
            .ok_or_else(|| format!("no agent process for pane {pane_id}"))?;
        process
            .interrupt()
            .map_err(|e| format!("failed to interrupt: {e}"))
    }
}

impl Default for AgentProcessStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Tauri commands ----

/// Spawn a new agent subprocess for a pane.
///
/// Resolves the backend config, spawns the process, registers it in the
/// agent registry, starts the stdout reader, and returns the instance ID.
#[tauri::command]
pub async fn spawn_agent(
    request: SpawnAgentRequest,
    registry: tauri::State<'_, Arc<AgentRegistry>>,
    process_store: tauri::State<'_, Arc<AgentProcessStore>>,
    window: tauri::WebviewWindow,
) -> Result<SpawnAgentResponse, String> {
    // Resolve backend config
    let mut config = resolve_backend_config(&request.backend_id)?;

    // If resuming a Claude Code session, add --resume flag
    if let Some(session_id) = &request.resume_session_id {
        if config.id == "claudecode" {
            config.args.push("--resume".to_string());
            config.args.push(session_id.clone());
        }
    }

    // Check if there's already an agent for this pane
    if let Some(existing) = registry.get(&request.pane_id) {
        if existing.status.is_running() {
            return Err(format!(
                "agent already running for pane {} (instance {})",
                request.pane_id, existing.instance_id
            ));
        }
        // Clean up terminated agent
        registry.remove(&request.pane_id);
        process_store.remove(&request.pane_id).await;
    }

    // Generate instance ID
    let instance_id = uuid::Uuid::new_v4().to_string();

    // Create agent state
    let mut state = AgentState::new(instance_id.clone(), request.pane_id.clone(), config.clone());
    state.set_starting();

    // Register before spawning (so status queries work immediately)
    registry
        .register(state)
        .map_err(|e| format!("failed to register agent: {e}"))?;

    // Spawn the subprocess
    let mut process = match AgentProcess::spawn(
        &config,
        request.cwd.as_deref(),
        &request.env,
    ) {
        Ok(p) => p,
        Err(e) => {
            registry.update(&request.pane_id, |s| {
                s.set_error(format!("spawn failed: {e}"));
            }).ok();
            return Err(format!("failed to spawn agent: {e}"));
        }
    };

    // Take stdout for the reader task
    let stdout = process.take_stdout().ok_or("agent process has no stdout")?;

    // Store the process handle
    process_store.insert(request.pane_id.clone(), process).await;

    // Update status to running
    registry
        .update(&request.pane_id, |s| {
            s.set_running();
        })
        .map_err(|e| format!("failed to update status: {e}"))?;

    // Start the stdout reader task — forwards events to frontend via Tauri events
    let pane_id = request.pane_id.clone();
    let inst_id = instance_id.clone();
    let reg = Arc::clone(&registry);
    let ps = Arc::clone(&process_store);
    let win = window.clone();

    let (tx, mut rx) = mpsc::channel::<AgentOutputEvent>(256);
    crate::backend::ai::process::spawn_stdout_reader(stdout, tx);

    // Event forwarding task
    tokio::spawn(async move {
        use tauri::Emitter;

        while let Some(event) = rx.recv().await {
            match &event {
                AgentOutputEvent::AdapterEvents { events } => {
                    // Extract session metadata from SessionStart events
                    for ev in events {
                        if let crate::backend::ai::adapters::AdapterEvent::SessionStart {
                            session_id,
                            model,
                            cwd,
                            ..
                        } = ev
                        {
                            reg.update(&pane_id, |s| {
                                s.session_meta
                                    .insert("session_id".to_string(), session_id.clone());
                                if let Some(m) = model {
                                    s.session_meta.insert("model".to_string(), m.clone());
                                }
                                if !cwd.is_empty() {
                                    s.session_meta.insert("cwd".to_string(), cwd.clone());
                                }
                            })
                            .ok();
                        }
                    }

                    // Forward adapter events to frontend
                    let _ = win.emit(
                        &format!("agent-output:{pane_id}"),
                        serde_json::json!({
                            "pane_id": pane_id,
                            "instance_id": inst_id,
                            "events": events,
                        }),
                    );
                }
                AgentOutputEvent::RawLine { line } => {
                    let _ = win.emit(
                        &format!("agent-raw:{pane_id}"),
                        serde_json::json!({
                            "pane_id": pane_id,
                            "line": line,
                        }),
                    );
                }
                AgentOutputEvent::StreamEnd => {
                    // Process exited — get exit code
                    let exit_code = if let Some(mut proc) = ps.remove(&pane_id).await {
                        proc.wait().await.unwrap_or(-1)
                    } else {
                        -1
                    };

                    reg.update(&pane_id, |s| {
                        s.set_done(exit_code);
                    })
                    .ok();

                    let _ = win.emit(
                        &format!("agent-status:{pane_id}"),
                        AgentStatusEvent {
                            pane_id: pane_id.clone(),
                            instance_id: inst_id.clone(),
                            status: AgentStatus::Done { exit_code },
                            error: None,
                        },
                    );
                    break;
                }
                AgentOutputEvent::ReadError { message } => {
                    reg.update(&pane_id, |s| {
                        s.set_error(message.clone());
                    })
                    .ok();

                    let _ = win.emit(
                        &format!("agent-status:{pane_id}"),
                        AgentStatusEvent {
                            pane_id: pane_id.clone(),
                            instance_id: inst_id.clone(),
                            status: AgentStatus::Error {
                                message: message.clone(),
                            },
                            error: Some(message.clone()),
                        },
                    );
                    break;
                }
            }
        }
    });

    // Send initial prompt if provided
    if let Some(prompt) = &request.initial_prompt {
        if !prompt.is_empty() {
            // Small delay to let the agent finish starting up
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let _ = process_store
                .write_stdin(&request.pane_id, prompt)
                .await;
        }
    }

    // Emit status event
    use tauri::Emitter;
    let _ = window.emit(
        &format!("agent-status:{}", request.pane_id),
        AgentStatusEvent {
            pane_id: request.pane_id.clone(),
            instance_id: instance_id.clone(),
            status: AgentStatus::Running,
            error: None,
        },
    );

    Ok(SpawnAgentResponse {
        instance_id,
        status: AgentStatus::Running,
    })
}

/// Send text input to a running agent's stdin.
#[tauri::command]
pub async fn send_agent_input(
    request: AgentInputRequest,
    registry: tauri::State<'_, Arc<AgentRegistry>>,
    process_store: tauri::State<'_, Arc<AgentProcessStore>>,
) -> Result<(), String> {
    // Validate agent exists and can accept input
    let state = registry
        .get(&request.pane_id)
        .ok_or_else(|| format!("no agent for pane {}", request.pane_id))?;

    if !state.can_accept_input() && !state.status.is_running() {
        return Err(format!(
            "agent for pane {} is not running (status: {:?})",
            request.pane_id, state.status
        ));
    }

    // Handle signal request
    if let Some(signal) = &request.signal {
        if signal == "SIGINT" {
            return process_store.interrupt(&request.pane_id).await;
        }
        return Err(format!("unsupported signal: {signal}"));
    }

    // Send text input
    if let Some(text) = &request.text {
        // Update status to busy
        registry
            .update(&request.pane_id, |s| {
                s.set_busy();
            })
            .ok();

        // Check if we should use structured input (Claude Code with session_id)
        if state.config.id == "claudecode" {
            if let Some(session_id) = state.session_meta.get("session_id") {
                // Use structured NDJSON format for follow-up messages
                return process_store
                    .send_structured_input(&request.pane_id, session_id, text)
                    .await;
            }
        }

        // Fallback: raw stdin write (for other backends or initial prompt)
        process_store.write_stdin(&request.pane_id, text).await
    } else {
        Err("either text or signal must be provided".into())
    }
}

/// Send SIGINT to interrupt a running agent.
#[tauri::command]
pub async fn interrupt_agent(
    pane_id: String,
    process_store: tauri::State<'_, Arc<AgentProcessStore>>,
) -> Result<(), String> {
    process_store.interrupt(&pane_id).await
}

/// Force-kill an agent subprocess.
#[tauri::command]
pub async fn kill_agent(
    pane_id: String,
    registry: tauri::State<'_, Arc<AgentRegistry>>,
    process_store: tauri::State<'_, Arc<AgentProcessStore>>,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    // Kill the process
    if let Some(mut process) = process_store.remove(&pane_id).await {
        process.kill().await.map_err(|e| format!("kill failed: {e}"))?;
    }

    // Update registry
    let instance_id = registry
        .update(&pane_id, |s| {
            s.set_done(-9); // -9 = killed
            s.instance_id.clone()
        })
        .unwrap_or_default();

    // Emit status event
    use tauri::Emitter;
    let _ = window.emit(
        &format!("agent-status:{pane_id}"),
        AgentStatusEvent {
            pane_id: pane_id.clone(),
            instance_id,
            status: AgentStatus::Done { exit_code: -9 },
            error: None,
        },
    );

    Ok(())
}

/// Get the current status of an agent for a pane.
#[tauri::command]
pub fn get_agent_status(
    pane_id: String,
    registry: tauri::State<'_, Arc<AgentRegistry>>,
) -> Result<AgentStatusResponse, String> {
    match registry.get(&pane_id) {
        Some(state) => Ok(AgentStatusResponse {
            instance_id: state.instance_id,
            status: state.status,
            backend_id: state.config.id,
        }),
        None => Ok(AgentStatusResponse {
            instance_id: String::new(),
            status: AgentStatus::Init,
            backend_id: String::new(),
        }),
    }
}

/// List available agent backends (auto-detected from PATH + built-in configs).
#[tauri::command]
pub fn list_agent_backends() -> Vec<AgentBackendConfig> {
    detect_available_backends()
}

// ---- Response types ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub instance_id: String,
    pub status: AgentStatus,
    pub backend_id: String,
}

// ---- Helpers ----

/// Resolve a backend ID to its configuration.
fn resolve_backend_config(backend_id: &str) -> Result<AgentBackendConfig, String> {
    match backend_id {
        "claudecode" => Ok(claude_code_config()),
        "gemini-cli" => Ok(gemini_cli_config()),
        "codex-cli" => Ok(codex_cli_config()),
        other => Err(format!("unknown agent backend: {other}")),
    }
}
