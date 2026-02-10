// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Tauri IPC bridge for RPC communication.
//!
//! Thin adapter layer — extracts parameters from IPC calls and delegates
//! to focused handler modules. Each sub-module owns a single domain concern.

mod block;
mod config;
mod events;
pub mod file;
mod object;
pub mod reactive;
pub mod schema;

use serde_json::Value;

use crate::state::AppState;

// Re-export Tauri commands that live in sub-modules
pub use file::fetch_wave_file;
pub use reactive::{reactive_inject, reactive_poller_config, reactive_register, reactive_unregister};
pub use schema::get_schema;

/// Handle an RPC request from the frontend.
///
/// Routes the message through the command dispatcher and returns the response.
#[tauri::command(rename_all = "camelCase")]
pub async fn rpc_request(
    msg: Value,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    dispatch_rpc(msg, &state)
}

/// Handle a backend service call from the frontend.
///
/// Processes the service call directly (replaces HTTP POST to /wave/service).
#[tauri::command(rename_all = "camelCase")]
pub async fn service_request(
    service: String,
    method: String,
    args: Value,
    ui_context: Option<Value>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let result = dispatch_service(&service, &method, &args, ui_context.as_ref(), &state)?;
    tracing::debug!("service_request: {}.{} => {:?}", service, method, result);
    Ok(result)
}

/// Direct Tauri command for terminal resize.
#[tauri::command(rename_all = "camelCase")]
pub async fn set_block_term_size(
    block_id: String,
    rows: i64,
    cols: i64,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let _ = &state;
    if rows <= 0 || cols <= 0 {
        return Err("invalid terminal dimensions".to_string());
    }
    let input = crate::backend::blockcontroller::BlockInputUnion::resize(
        crate::backend::waveobj::TermSize { rows, cols },
    );
    crate::backend::blockcontroller::send_input(&block_id, input)
        .map_err(|e| format!("set_block_term_size: {}", e))
}

// ---- Dispatchers ----

/// RPC command dispatcher — routes by command name to handler modules.
fn dispatch_rpc(msg: Value, state: &AppState) -> Result<Value, String> {
    let command = msg
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let data = msg.get("data").cloned().unwrap_or(Value::Null);
    let reqid = msg
        .get("reqid")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    tracing::debug!("rpc_request: command={}, reqid={}", command, reqid);

    let result = match command {
        // Route management
        "routeannounce" | "routeunannounce" => Ok(Value::Null),

        // Config
        "getfullconfig" => {
            let config = state.config_watcher.get_full_config();
            serde_json::to_value(&*config).map_err(|e| format!("serialize config: {}", e))
        }
        "setconfig" => config::handle_set_config(&data, state),
        "setconnectionsconfig" => config::handle_set_connections_config(&data, state),

        // Object metadata
        "getmeta" => object::handle_get_meta(&data, state),
        "setmeta" => object::handle_set_meta(&data, state),

        // Events
        "eventsub" => events::handle_event_sub(&data, state),
        "eventpublish" => events::handle_event_publish(&data, state),
        "resolveids" => events::handle_resolve_ids(&data, state),

        // Blocks and controllers
        "createblock" => block::handle_create_block(&data, state),
        "deleteblock" => block::handle_delete_block(&data, state),
        "controllerinput" => block::handle_controller_input(&data, state),
        "controllerresync" => block::handle_controller_resync(&data, state),
        "setblocktermsize" => block::handle_set_block_term_size(&data, state),

        // Status/updates (return empty for now)
        "controllerstatusupdates" | "getallupdates" | "getallobj" => {
            Ok(serde_json::json!({ "updates": [] }))
        }

        _ => {
            tracing::warn!("unhandled rpc command: {}", command);
            Ok(Value::Null)
        }
    };

    // Wrap result with reqid for the frontend
    match result {
        Ok(response_data) => {
            if reqid.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(serde_json::json!({
                    "resid": reqid,
                    "data": response_data,
                }))
            }
        }
        Err(e) => {
            if reqid.is_empty() {
                Err(e)
            } else {
                Ok(serde_json::json!({
                    "resid": reqid,
                    "error": e,
                }))
            }
        }
    }
}

/// Service request dispatcher — routes by service.method to handler modules.
fn dispatch_service(
    service: &str,
    method: &str,
    args: &Value,
    _ui_context: Option<&Value>,
    state: &AppState,
) -> Result<Value, String> {
    match service {
        "client" => object::handle_client_service(method, state),
        "window" => object::handle_window_service(method, args, state),
        "workspace" => object::handle_workspace_service(method, args, state),
        "object" => object::handle_object_service(method, args, state),
        _ => {
            tracing::warn!("unhandled service call: {}.{}", service, method);
            Ok(serde_json::json!({ "data": null, "updates": [] }))
        }
    }
}
