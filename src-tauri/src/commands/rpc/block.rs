// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Block and terminal controller handlers.

use serde_json::Value;

use crate::state::AppState;

/// Handle createblock RPC command.
pub fn handle_create_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state
        .active_tab_id
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let store = &state.wave_store;

    let meta: crate::backend::waveobj::MetaMapType = data
        .get("meta")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let block = crate::backend::wcore::create_block(store, &tab_id, meta)
        .map_err(|e| format!("createblock: {}", e))?;

    // If the block has a controller (shell/cmd), start it via resync
    let controller_type = block
        .meta
        .get("controller")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !controller_type.is_empty() {
        let broker = Some(std::sync::Arc::clone(&state.broker));
        if let Err(e) =
            crate::backend::blockcontroller::resync_controller(&block, &tab_id, None, broker, false)
        {
            tracing::warn!(
                "Failed to start controller for block {}: {}",
                block.oid,
                e
            );
        }
    }

    Ok(serde_json::json!({
        "otype": "block",
        "oid": block.oid,
    }))
}

/// Handle deleteblock RPC command.
pub fn handle_delete_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state
        .active_tab_id
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let block_id = data
        .get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "deleteblock: missing blockid".to_string())?;

    // Stop and remove any controller for this block
    crate::backend::blockcontroller::stop_block_controller(block_id).ok();
    crate::backend::blockcontroller::delete_controller(block_id);

    let store = &state.wave_store;
    crate::backend::wcore::delete_block(store, &tab_id, block_id)
        .map_err(|e| format!("deleteblock: {}", e))?;

    Ok(Value::Null)
}

/// Handle controllerinput RPC command.
/// Frontend sends: { blockid, inputdata64?, signame?, termsize? }
pub fn handle_controller_input(data: &Value, _state: &AppState) -> Result<Value, String> {
    use base64::Engine as _;
    let block_id = data
        .get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "controllerinput: missing blockid".to_string())?;

    let mut input = crate::backend::blockcontroller::BlockInputUnion {
        input_data: None,
        sig_name: None,
        term_size: None,
    };

    // Decode base64 input data
    if let Some(data64) = data.get("inputdata64").and_then(|v| v.as_str()) {
        if !data64.is_empty() {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(data64)
                .map_err(|e| format!("controllerinput: invalid base64: {}", e))?;
            input.input_data = Some(decoded);
        }
    }

    // Signal name
    if let Some(sig) = data.get("signame").and_then(|v| v.as_str()) {
        if !sig.is_empty() {
            input.sig_name = Some(sig.to_string());
        }
    }

    // Terminal resize
    if let Some(ts) = data.get("termsize") {
        let rows = ts.get("rows").and_then(|v| v.as_i64()).unwrap_or(0);
        let cols = ts.get("cols").and_then(|v| v.as_i64()).unwrap_or(0);
        if rows > 0 && cols > 0 {
            input.term_size = Some(crate::backend::waveobj::TermSize { rows, cols });
        }
    }

    crate::backend::blockcontroller::send_input(block_id, input)
        .map_err(|e| format!("controllerinput: {}", e))?;

    Ok(Value::Null)
}

/// Handle controllerresync RPC command.
/// Frontend sends: { blockid, tabid, forcerestart?, rtopts? }
pub fn handle_controller_resync(data: &Value, state: &AppState) -> Result<Value, String> {
    let block_id = data
        .get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "controllerresync: missing blockid".to_string())?;

    let tab_id = data
        .get("tabid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            state
                .active_tab_id
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default()
        });

    let force = data
        .get("forcerestart")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let rt_opts = data.get("rtopts").cloned();

    let store = &state.wave_store;
    let block = store
        .must_get::<crate::backend::waveobj::Block>(block_id)
        .map_err(|e| format!("controllerresync: {}", e))?;

    let broker = Some(std::sync::Arc::clone(&state.broker));
    crate::backend::blockcontroller::resync_controller(&block, &tab_id, rt_opts, broker, force)
        .map_err(|e| format!("controllerresync: {}", e))?;

    Ok(Value::Null)
}

/// Handle setblocktermsize RPC command.
/// Frontend sends: { blockid, termsize: { rows, cols } }
pub fn handle_set_block_term_size(data: &Value, _state: &AppState) -> Result<Value, String> {
    let block_id = data
        .get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "setblocktermsize: missing blockid".to_string())?;

    let ts = data
        .get("termsize")
        .ok_or_else(|| "setblocktermsize: missing termsize".to_string())?;
    let rows = ts.get("rows").and_then(|v| v.as_i64()).unwrap_or(0);
    let cols = ts.get("cols").and_then(|v| v.as_i64()).unwrap_or(0);

    if rows <= 0 || cols <= 0 {
        return Err("setblocktermsize: invalid dimensions".to_string());
    }

    let input = crate::backend::blockcontroller::BlockInputUnion::resize(
        crate::backend::waveobj::TermSize { rows, cols },
    );

    crate::backend::blockcontroller::send_input(block_id, input)
        .map_err(|e| format!("setblocktermsize: {}", e))?;

    Ok(Value::Null)
}
