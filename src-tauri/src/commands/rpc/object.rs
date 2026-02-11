// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Object CRUD handlers — GetObject, UpdateObject, GetMeta, SetMeta.

use serde_json::Value;

use crate::backend::storage::wstore::WaveStore;
use crate::backend::waveobj::*;
use crate::state::AppState;

/// Handle getmeta RPC command.
pub fn handle_get_meta(data: &Value, state: &AppState) -> Result<Value, String> {
    let oref_str = data
        .get("oref")
        .or_else(|| data.as_str().map(|_| data))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "getmeta: missing oref".to_string())?;

    let store = &state.wave_store;
    let parts: Vec<&str> = oref_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid oref: {}", oref_str));
    }

    let (otype, oid) = (parts[0], parts[1]);
    get_obj_json(store, otype, oid).map(|obj| obj.get("meta").cloned().unwrap_or(Value::Null))
}

/// Handle setmeta RPC command.
pub fn handle_set_meta(data: &Value, state: &AppState) -> Result<Value, String> {
    let oref_str = data
        .get("oref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "setmeta: missing oref".to_string())?;
    let meta = data
        .get("meta")
        .ok_or_else(|| "setmeta: missing meta".to_string())?;

    let store = &state.wave_store;
    let parts: Vec<&str> = oref_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid oref: {}", oref_str));
    }

    let (otype, oid) = (parts[0], parts[1]);

    match otype {
        "block" => {
            let mut obj = store
                .must_get::<Block>(oid)
                .map_err(|e| format!("setmeta block: {}", e))?;
            if let Some(meta_map) = meta.as_object() {
                for (k, v) in meta_map {
                    obj.meta.insert(k.clone(), v.clone());
                }
            }
            store
                .update(&mut obj)
                .map_err(|e| format!("setmeta update: {}", e))?;
        }
        "tab" => {
            let mut obj = store
                .must_get::<Tab>(oid)
                .map_err(|e| format!("setmeta tab: {}", e))?;
            if let Some(meta_map) = meta.as_object() {
                for (k, v) in meta_map {
                    obj.meta.insert(k.clone(), v.clone());
                }
            }
            store
                .update(&mut obj)
                .map_err(|e| format!("setmeta update: {}", e))?;
        }
        _ => {
            tracing::warn!("setmeta: unsupported otype {}", otype);
        }
    }

    Ok(Value::Null)
}

/// Handle service_request for object-related methods.
pub fn handle_object_service(
    method: &str,
    args: &Value,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match method {
        "GetObject" => {
            let oref = args
                .get(0)
                .and_then(|v| v.as_str())
                .ok_or_else(|| "GetObject: missing oref arg".to_string())?;
            let parts: Vec<&str> = oref.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("invalid oref: {}", oref));
            }
            let obj = get_obj_json(store, parts[0], parts[1])?;
            Ok(serde_json::json!({ "data": obj, "updates": [] }))
        }

        "UpdateObject" => {
            let obj_data = args
                .get(0)
                .ok_or_else(|| "UpdateObject: missing obj arg".to_string())?;
            let otype = obj_data
                .get("otype")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match otype {
                "block" => {
                    let mut obj: Block = serde_json::from_value(obj_data.clone())
                        .map_err(|e| format!("UpdateObject block parse: {}", e))?;
                    store
                        .update(&mut obj)
                        .map_err(|e| format!("UpdateObject block: {}", e))?;
                }
                "tab" => {
                    let mut obj: Tab = serde_json::from_value(obj_data.clone())
                        .map_err(|e| format!("UpdateObject tab parse: {}", e))?;
                    store
                        .update(&mut obj)
                        .map_err(|e| format!("UpdateObject tab: {}", e))?;
                }
                _ => {
                    tracing::warn!("UpdateObject: unsupported otype {}", otype);
                }
            }

            Ok(serde_json::json!({ "data": true, "updates": [] }))
        }

        _ => Err(format!("unknown object method: {}", method)),
    }
}

/// Handle service_request for client-related methods.
pub fn handle_client_service(
    method: &str,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match method {
        "GetClientData" => {
            let client = crate::backend::wcore::get_client(store)
                .map_err(|e| format!("GetClientData: {}", e))?;
            let client_json = serde_json::to_value(&client)
                .map_err(|e| format!("GetClientData serialize: {}", e))?;
            Ok(serde_json::json!({ "data": client_json, "updates": [] }))
        }
        "GetAllConnStatus" => {
            // No SSH/WSL connection management in Rust backend yet — return empty array
            Ok(serde_json::json!({ "data": [], "updates": [] }))
        }
        _ => Err(format!("unknown client method: {}", method)),
    }
}

/// Handle service_request for window-related methods.
pub fn handle_window_service(
    method: &str,
    args: &Value,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match method {
        "GetWindow" => {
            let window_id = args
                .get(0)
                .and_then(|v| v.as_str())
                .ok_or_else(|| "GetWindow: missing windowId arg".to_string())?;
            let window = store
                .must_get::<Window>(window_id)
                .map_err(|e| format!("GetWindow: {}", e))?;
            let window_json = serde_json::to_value(&window)
                .map_err(|e| format!("GetWindow serialize: {}", e))?;
            Ok(serde_json::json!({ "data": window_json, "updates": [] }))
        }
        _ => Err(format!("unknown window method: {}", method)),
    }
}

/// Handle service_request for workspace-related methods.
pub fn handle_workspace_service(
    method: &str,
    args: &Value,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match method {
        "GetWorkspace" => {
            let workspace_id = args
                .get(0)
                .and_then(|v| v.as_str())
                .ok_or_else(|| "GetWorkspace: missing workspaceId arg".to_string())?;
            let workspace = crate::backend::wcore::get_workspace(store, workspace_id)
                .map_err(|e| format!("GetWorkspace: {}", e))?;
            let workspace_json = serde_json::to_value(&workspace)
                .map_err(|e| format!("GetWorkspace serialize: {}", e))?;
            Ok(serde_json::json!({ "data": workspace_json, "updates": [] }))
        }
        _ => Err(format!("unknown workspace method: {}", method)),
    }
}

/// Get a WaveObj as JSON by otype/oid.
/// Adds the "otype" field to the JSON response since it's not part of the struct fields.
pub fn get_obj_json(store: &WaveStore, otype: &str, oid: &str) -> Result<Value, String> {
    let mut obj_json = match otype {
        OTYPE_CLIENT => store
            .must_get::<Client>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        OTYPE_WINDOW => store
            .must_get::<Window>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        OTYPE_WORKSPACE => store
            .must_get::<Workspace>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        OTYPE_TAB => store
            .must_get::<Tab>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        OTYPE_LAYOUT => store
            .must_get::<LayoutState>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        OTYPE_BLOCK => store
            .must_get::<Block>(oid)
            .map(|o| serde_json::to_value(&o).unwrap_or(Value::Null))
            .map_err(|e| format!("get {}: {}", otype, e))?,
        _ => return Err(format!("unknown otype: {}", otype)),
    };

    // Frontend expects "otype" field in the JSON
    if let Value::Object(ref mut map) = obj_json {
        map.insert("otype".to_string(), Value::String(otype.to_string()));
    }

    Ok(obj_json)
}
