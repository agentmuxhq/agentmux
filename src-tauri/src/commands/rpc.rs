// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Tauri IPC bridge for RPC communication.
// Replaces WebSocket (ws://127.0.0.1:8877/ws) and HTTP service
// (http://127.0.0.1:8876/wave/service) calls in rust-backend mode.
//
// The frontend sends RpcMessages via invoke("rpc_request"),
// and receives events via Tauri's event system (emit).

use serde_json::Value;

use crate::state::AppState;

/// Handle an RPC request from the frontend.
///
/// In rust-backend mode: routes the message through the WshRpcEngine
/// and returns the response synchronously.
/// In go-sidecar mode: not used (frontend talks via WebSocket).
#[tauri::command(rename_all = "camelCase")]
pub async fn rpc_request(
    msg: Value,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    #[cfg(feature = "rust-backend")]
    {
        return handle_rpc_request(msg, &state).await;
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = (msg, state);
        Err("rpc_request only available in rust-backend mode".to_string())
    }
}

/// Handle a backend service call from the frontend.
///
/// In rust-backend mode: processes the service call directly (replaces
/// HTTP POST to /wave/service).
/// In go-sidecar mode: not used (frontend talks via HTTP).
#[tauri::command(rename_all = "camelCase")]
pub async fn service_request(
    service: String,
    method: String,
    args: Value,
    ui_context: Option<Value>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    #[cfg(feature = "rust-backend")]
    {
        return handle_service_request(&service, &method, &args, ui_context.as_ref(), &state);
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = (service, method, args, ui_context, state);
        Err("service_request only available in rust-backend mode".to_string())
    }
}

// ---- rust-backend implementations ----

#[cfg(feature = "rust-backend")]
async fn handle_rpc_request(
    msg: Value,
    state: &AppState,
) -> Result<Value, String> {
    use crate::backend::waveobj::*;

    let command = msg.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let data = msg.get("data").cloned().unwrap_or(Value::Null);
    let reqid = msg.get("reqid").and_then(|v| v.as_str()).unwrap_or("");

    tracing::debug!("rpc_request: command={}, reqid={}", command, reqid);

    let result = match command {
        "routeannounce" | "routeunannounce" => {
            // Route management — acknowledge silently
            Ok(Value::Null)
        }

        "getfullconfig" => {
            // Return a minimal config for now
            Ok(serde_json::json!({
                "settings": {},
                "presets": {},
                "termthemes": {},
                "mimetofileext": {},
            }))
        }

        "getmeta" => {
            handle_get_meta(&data, state)
        }

        "setmeta" => {
            handle_set_meta(&data, state)
        }

        "eventsub" => {
            // Event subscription — store in broker
            handle_event_sub(&data, state)
        }

        "eventpublish" => {
            handle_event_publish(&data, state)
        }

        "resolveids" => {
            handle_resolve_ids(&data, state)
        }

        "createblock" => {
            handle_create_block(&data, state)
        }

        "deleteblock" => {
            handle_delete_block(&data, state)
        }

        "getallupdates" | "getallobj" => {
            // Return empty updates for now
            Ok(serde_json::json!({ "updates": [] }))
        }

        _ => {
            tracing::warn!("unhandled rpc command: {}", command);
            Ok(Value::Null)
        }
    };

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

#[cfg(feature = "rust-backend")]
fn handle_get_meta(data: &Value, state: &AppState) -> Result<Value, String> {
    let oref_str = data.get("oref")
        .or_else(|| data.as_str().map(|_| data))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "getmeta: missing oref".to_string())?;

    let store = &state.wave_store;
    let parts: Vec<&str> = oref_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid oref: {}", oref_str));
    }

    let (otype, oid) = (parts[0], parts[1]);
    get_obj_json(store, otype, oid)
        .map(|obj| obj.get("meta").cloned().unwrap_or(Value::Null))
}

#[cfg(feature = "rust-backend")]
fn handle_set_meta(data: &Value, state: &AppState) -> Result<Value, String> {
    let oref_str = data.get("oref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "setmeta: missing oref".to_string())?;
    let meta = data.get("meta")
        .ok_or_else(|| "setmeta: missing meta".to_string())?;

    let store = &state.wave_store;
    let parts: Vec<&str> = oref_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid oref: {}", oref_str));
    }

    let (otype, oid) = (parts[0], parts[1]);

    // Update the meta field on the object
    match otype {
        "block" => {
            let mut obj = store.must_get::<crate::backend::waveobj::Block>(oid)
                .map_err(|e| format!("setmeta block: {}", e))?;
            if let Some(meta_map) = meta.as_object() {
                for (k, v) in meta_map {
                    obj.meta.insert(k.clone(), v.clone());
                }
            }
            store.update(&mut obj).map_err(|e| format!("setmeta update: {}", e))?;
        }
        "tab" => {
            let mut obj = store.must_get::<crate::backend::waveobj::Tab>(oid)
                .map_err(|e| format!("setmeta tab: {}", e))?;
            if let Some(meta_map) = meta.as_object() {
                for (k, v) in meta_map {
                    obj.meta.insert(k.clone(), v.clone());
                }
            }
            store.update(&mut obj).map_err(|e| format!("setmeta update: {}", e))?;
        }
        _ => {
            tracing::warn!("setmeta: unsupported otype {}", otype);
        }
    }

    Ok(Value::Null)
}

#[cfg(feature = "rust-backend")]
fn handle_event_sub(data: &Value, state: &AppState) -> Result<Value, String> {
    // Register the subscription in the broker
    let event_type = data.get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let scopes: Vec<String> = data.get("scopes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    tracing::debug!("eventsub: event={}, scopes={:?}", event_type, scopes);

    // Subscribe in the broker
    let sub = crate::backend::wps::SubscriptionRequest {
        event: event_type.to_string(),
        scopes,
        allscopes: data.get("allscopes").and_then(|v| v.as_bool()).unwrap_or(false),
    };
    state.broker.subscribe("frontend", sub);

    Ok(Value::Null)
}

#[cfg(feature = "rust-backend")]
fn handle_event_publish(data: &Value, state: &AppState) -> Result<Value, String> {
    let event = serde_json::from_value::<crate::backend::wps::WaveEvent>(data.clone())
        .map_err(|e| format!("eventpublish: invalid event: {}", e))?;
    state.broker.publish(event);
    Ok(Value::Null)
}

#[cfg(feature = "rust-backend")]
fn handle_resolve_ids(data: &Value, state: &AppState) -> Result<Value, String> {
    // Return client/window/workspace/tab IDs
    let client_id = state.client_id.lock().unwrap().clone().unwrap_or_default();
    let window_id = state.window_id.lock().unwrap().clone().unwrap_or_default();
    let active_tab_id = state.active_tab_id.lock().unwrap().clone().unwrap_or_default();

    let store = &state.wave_store;

    // Get workspace from window
    let workspace_id = if !window_id.is_empty() {
        store.must_get::<crate::backend::waveobj::Window>(&window_id)
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

#[cfg(feature = "rust-backend")]
fn handle_create_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state.active_tab_id.lock().unwrap().clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let store = &state.wave_store;

    let meta: crate::backend::waveobj::MetaMapType = data.get("meta")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let block = crate::backend::wcore::create_block(store, &tab_id, meta)
        .map_err(|e| format!("createblock: {}", e))?;

    Ok(serde_json::json!({
        "otype": "block",
        "oid": block.oid,
    }))
}

#[cfg(feature = "rust-backend")]
fn handle_delete_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state.active_tab_id.lock().unwrap().clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let block_id = data.get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "deleteblock: missing blockid".to_string())?;

    let store = &state.wave_store;
    crate::backend::wcore::delete_block(store, &tab_id, block_id)
        .map_err(|e| format!("deleteblock: {}", e))?;

    Ok(Value::Null)
}

#[cfg(feature = "rust-backend")]
fn handle_service_request(
    service: &str,
    method: &str,
    args: &Value,
    _ui_context: Option<&Value>,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match (service, method) {
        ("object", "GetObject") => {
            let oref = args.get(0)
                .and_then(|v| v.as_str())
                .ok_or_else(|| "GetObject: missing oref arg".to_string())?;
            let parts: Vec<&str> = oref.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("invalid oref: {}", oref));
            }
            let obj = get_obj_json(store, parts[0], parts[1])?;
            Ok(serde_json::json!({
                "data": obj,
                "updates": [],
            }))
        }

        ("object", "UpdateObject") => {
            let obj_data = args.get(0)
                .ok_or_else(|| "UpdateObject: missing obj arg".to_string())?;
            let otype = obj_data.get("otype").and_then(|v| v.as_str()).unwrap_or("");
            let oid = obj_data.get("oid").and_then(|v| v.as_str()).unwrap_or("");

            // Update the object in the store
            match otype {
                "block" => {
                    let mut obj: crate::backend::waveobj::Block = serde_json::from_value(obj_data.clone())
                        .map_err(|e| format!("UpdateObject block parse: {}", e))?;
                    store.update(&mut obj).map_err(|e| format!("UpdateObject block: {}", e))?;
                }
                "tab" => {
                    let mut obj: crate::backend::waveobj::Tab = serde_json::from_value(obj_data.clone())
                        .map_err(|e| format!("UpdateObject tab parse: {}", e))?;
                    store.update(&mut obj).map_err(|e| format!("UpdateObject tab: {}", e))?;
                }
                _ => {
                    tracing::warn!("UpdateObject: unsupported otype {}", otype);
                }
            }

            Ok(serde_json::json!({
                "data": true,
                "updates": [],
            }))
        }

        _ => {
            tracing::warn!("unhandled service call: {}.{}", service, method);
            Ok(serde_json::json!({
                "data": null,
                "updates": [],
            }))
        }
    }
}

/// Helper: get a WaveObj as JSON by otype/oid.
#[cfg(feature = "rust-backend")]
fn get_obj_json(
    store: &crate::backend::storage::wstore::WaveStore,
    otype: &str,
    oid: &str,
) -> Result<Value, String> {
    use crate::backend::waveobj::*;

    match otype {
        OTYPE_CLIENT => store.must_get::<Client>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        OTYPE_WINDOW => store.must_get::<Window>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        OTYPE_WORKSPACE => store.must_get::<Workspace>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        OTYPE_TAB => store.must_get::<Tab>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        OTYPE_LAYOUT => store.must_get::<LayoutState>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        OTYPE_BLOCK => store.must_get::<Block>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e)),
        _ => Err(format!("unknown otype: {}", otype)),
    }
}
