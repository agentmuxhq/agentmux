// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Event subscription, publish, and ID resolution handlers.

use serde_json::Value;

use crate::state::AppState;

/// Handle eventsub RPC command — register event subscription in broker.
pub fn handle_event_sub(data: &Value, state: &AppState) -> Result<Value, String> {
    let event_type = data
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let scopes: Vec<String> = data
        .get("scopes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    tracing::debug!("eventsub: event={}, scopes={:?}", event_type, scopes);

    let sub = crate::backend::wps::SubscriptionRequest {
        event: event_type.to_string(),
        scopes,
        allscopes: data
            .get("allscopes")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };
    state.broker.subscribe("frontend", sub);

    Ok(Value::Null)
}

/// Handle eventpublish RPC command.
pub fn handle_event_publish(data: &Value, state: &AppState) -> Result<Value, String> {
    let event = serde_json::from_value::<crate::backend::wps::WaveEvent>(data.clone())
        .map_err(|e| format!("eventpublish: invalid event: {}", e))?;
    state.broker.publish(event);
    Ok(Value::Null)
}

/// Handle resolveids RPC command — return client/window/workspace/tab IDs.
pub fn handle_resolve_ids(_data: &Value, state: &AppState) -> Result<Value, String> {
    let client_id = state
        .client_id
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default();
    let window_id = state
        .window_id
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default();
    let active_tab_id = state
        .active_tab_id
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default();

    let store = &state.wave_store;

    let workspace_id = if !window_id.is_empty() {
        store
            .must_get::<crate::backend::waveobj::Window>(&window_id)
            .map(|w| w.workspaceid.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };

    Ok(serde_json::json!({
        "clientid": client_id,
        "windowid": window_id,
        "workspaceid": workspace_id,
        "tabid": active_tab_id,
    }))
}
