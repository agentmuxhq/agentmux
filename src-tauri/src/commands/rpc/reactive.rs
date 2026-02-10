// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Reactive messaging handlers — agent registration, injection, poller config.

use serde_json::Value;

use crate::state::AppState;

/// Register an agent with the reactive messaging handler.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_register(
    block_id: String,
    agent_id: String,
    tab_id: Option<String>,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let handler = crate::backend::reactive::get_global_handler();

    handler.set_input_sender(std::sync::Arc::new(|block_id: &str, data: &[u8]| {
        let input = crate::backend::blockcontroller::BlockInputUnion::data(data.to_vec());
        crate::backend::blockcontroller::send_input(block_id, input)
    }));

    handler
        .register_agent(&agent_id, &block_id, tab_id.as_deref())
        .map_err(|e| format!("register agent: {}", e))?;

    tracing::info!(
        "reactive: registered agent {} -> block {}",
        agent_id,
        block_id
    );
    Ok(serde_json::json!({"status": "ok"}))
}

/// Unregister an agent from the reactive messaging handler.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_unregister(
    agent_id: String,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let handler = crate::backend::reactive::get_global_handler();
    handler.unregister_agent(&agent_id);
    tracing::info!("reactive: unregistered agent {}", agent_id);
    Ok(serde_json::json!({"status": "ok"}))
}

/// Inject a message into a target agent's terminal.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_inject(
    target_agent: String,
    message: String,
    source_agent: Option<String>,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let handler = crate::backend::reactive::get_global_handler();
    let req = crate::backend::reactive::InjectionRequest {
        target_agent,
        message,
        source_agent,
        request_id: None,
        priority: None,
        wait_for_idle: false,
    };
    let resp = handler.inject_message(req);
    serde_json::to_value(&resp).map_err(|e| format!("serialize response: {}", e))
}

/// Configure the AgentBus poller for cross-host reactive messaging.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_poller_config(
    agentbus_url: Option<String>,
    agentbus_token: Option<String>,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    if let Some(ref url) = agentbus_url {
        if !url.is_empty() {
            crate::backend::reactive::validate_agentbus_url(url)?;
        }
    }

    let handler = crate::backend::reactive::get_global_handler();
    let poller = crate::backend::reactive::Poller::new(
        crate::backend::reactive::PollerConfig {
            agentbus_url: agentbus_url.clone(),
            agentbus_token: agentbus_token.clone(),
            poll_interval_secs: crate::backend::reactive::DEFAULT_POLL_INTERVAL_SECS,
        },
        handler,
    );

    let is_configured = poller.is_configured();
    tracing::info!(
        "reactive: poller config updated, configured={}",
        is_configured
    );

    Ok(serde_json::json!({
        "configured": is_configured,
        "running": false,
    }))
}
