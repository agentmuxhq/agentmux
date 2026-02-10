// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Config read/write handlers.

use serde_json::Value;

use crate::state::AppState;

/// Handle setconfig RPC command — merge values into settings.json.
pub fn handle_set_config(data: &Value, state: &AppState) -> Result<Value, String> {
    let to_merge = data
        .as_object()
        .ok_or_else(|| "setconfig: expected object".to_string())?;

    crate::backend::wconfig::set_base_config_value(&state.config_dir, to_merge)?;

    // Reload and broadcast
    let new_config = crate::backend::wconfig::load_full_config(&state.config_dir);
    state.config_watcher.set_config(new_config);
    state.broker.publish(crate::backend::wps::WaveEvent {
        event: crate::backend::wps::EVENT_CONFIG.to_string(),
        scopes: vec![],
        sender: String::new(),
        persist: 0,
        data: None,
    });

    Ok(Value::Null)
}

/// Handle setconnectionsconfig RPC command — merge values for a specific connection.
pub fn handle_set_connections_config(data: &Value, state: &AppState) -> Result<Value, String> {
    let conn_name = data
        .get("conn")
        .or_else(|| data.get("host"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "setconnectionsconfig: missing conn/host".to_string())?;

    let meta_map = data
        .get("metamaptype")
        .or_else(|| data.get("meta"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| "setconnectionsconfig: missing metamaptype/meta".to_string())?;

    crate::backend::wconfig::set_connections_config_value(
        &state.config_dir,
        conn_name,
        meta_map,
    )?;

    // Reload and broadcast
    let new_config = crate::backend::wconfig::load_full_config(&state.config_dir);
    state.config_watcher.set_config(new_config);
    state.broker.publish(crate::backend::wps::WaveEvent {
        event: crate::backend::wps::EVENT_CONFIG.to_string(),
        scopes: vec![],
        sender: String::new(),
        persist: 0,
        data: None,
    });

    Ok(Value::Null)
}
