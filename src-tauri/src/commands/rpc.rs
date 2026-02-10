// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Tauri IPC bridge for RPC communication.
// The frontend sends RpcMessages via invoke("rpc_request"),
// and receives events via Tauri's event system (emit).

use serde_json::Value;

use crate::state::AppState;

/// Handle an RPC request from the frontend.
///
/// Routes the message through the WshRpcEngine and returns the response.
#[tauri::command(rename_all = "camelCase")]
pub async fn rpc_request(
    msg: Value,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    handle_rpc_request(msg, &state).await
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
    handle_service_request(&service, &method, &args, ui_context.as_ref(), &state)
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

// ---- RPC request handler ----

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
            let config = state.config_watcher.get_full_config();
            serde_json::to_value(&*config)
                .map_err(|e| format!("serialize config: {}", e))
        }

        "setconfig" => {
            handle_set_config(&data, state)
        }

        "setconnectionsconfig" => {
            handle_set_connections_config(&data, state)
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

        "controllerinput" => {
            handle_controller_input(&data, state)
        }

        "controllerresync" => {
            handle_controller_resync(&data, state)
        }

        "setblocktermsize" => {
            handle_set_block_term_size(&data, state)
        }

        "controllerstatusupdates" | "getallupdates" | "getallobj" => {
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

fn handle_event_publish(data: &Value, state: &AppState) -> Result<Value, String> {
    let event = serde_json::from_value::<crate::backend::wps::WaveEvent>(data.clone())
        .map_err(|e| format!("eventpublish: invalid event: {}", e))?;
    state.broker.publish(event);
    Ok(Value::Null)
}

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

fn handle_create_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state.active_tab_id.lock().unwrap().clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let store = &state.wave_store;

    let meta: crate::backend::waveobj::MetaMapType = data.get("meta")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let block = crate::backend::wcore::create_block(store, &tab_id, meta)
        .map_err(|e| format!("createblock: {}", e))?;

    // If the block has a controller (shell/cmd), start it via resync
    let controller_type = block.meta.get("controller")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !controller_type.is_empty() {
        let broker = Some(std::sync::Arc::clone(&state.broker));
        if let Err(e) = crate::backend::blockcontroller::resync_controller(
            &block, &tab_id, None, broker, false,
        ) {
            tracing::warn!("Failed to start controller for block {}: {}", block.oid, e);
        }
    }

    Ok(serde_json::json!({
        "otype": "block",
        "oid": block.oid,
    }))
}

fn handle_delete_block(data: &Value, state: &AppState) -> Result<Value, String> {
    let tab_id = state.active_tab_id.lock().unwrap().clone()
        .ok_or_else(|| "no active tab".to_string())?;
    let block_id = data.get("blockid")
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

/// Handle terminal input from the frontend.
/// Frontend sends: { blockid, inputdata64?, signame?, termsize? }
fn handle_controller_input(data: &Value, _state: &AppState) -> Result<Value, String> {
    use base64::Engine as _;
    let block_id = data.get("blockid")
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

/// Handle controller resync (start/restart a block's shell process).
/// Frontend sends: { blockid, tabid, forcerestart?, rtopts? }
fn handle_controller_resync(data: &Value, state: &AppState) -> Result<Value, String> {
    let block_id = data.get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "controllerresync: missing blockid".to_string())?;

    let tab_id = data.get("tabid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            state.active_tab_id.lock().unwrap().clone().unwrap_or_default()
        });

    let force = data.get("forcerestart")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let rt_opts = data.get("rtopts").cloned();

    // Load the block from store
    let store = &state.wave_store;
    let block = store.must_get::<crate::backend::waveobj::Block>(block_id)
        .map_err(|e| format!("controllerresync: {}", e))?;

    let broker = Some(std::sync::Arc::clone(&state.broker));
    crate::backend::blockcontroller::resync_controller(&block, &tab_id, rt_opts, broker, force)
        .map_err(|e| format!("controllerresync: {}", e))?;

    Ok(Value::Null)
}

/// Handle terminal resize.
/// Frontend sends: { blockid, termsize: { rows, cols } }
fn handle_set_block_term_size(data: &Value, _state: &AppState) -> Result<Value, String> {
    let block_id = data.get("blockid")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "setblocktermsize: missing blockid".to_string())?;

    let ts = data.get("termsize")
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

fn handle_service_request(
    service: &str,
    method: &str,
    args: &Value,
    _ui_context: Option<&Value>,
    state: &AppState,
) -> Result<Value, String> {
    let store = &state.wave_store;

    match (service, method) {
        ("client", "GetClientData") => {
            let client = crate::backend::wcore::get_client(store)
                .map_err(|e| format!("GetClientData: {}", e))?;
            Ok(serde_json::to_value(&client)
                .map_err(|e| format!("GetClientData serialize: {}", e))?)
        }

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

// ---- File and reactive Tauri commands ----

/// Fetch a wave file's data and metadata.
#[tauri::command(rename_all = "camelCase")]
pub async fn fetch_wave_file(
    zone_id: String,
    name: String,
    offset: Option<i64>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    use base64::Engine as _;
    let file_store = &state.file_store;

    // Get file info
    let file_info = file_store
        .stat(&zone_id, &name)
        .map_err(|e| format!("stat: {}", e))?;

    let file_info = match file_info {
        Some(f) => f,
        None => {
            return Ok(serde_json::json!({
                "data": null,
                "fileInfo": null,
            }));
        }
    };

    // Read data
    let data_bytes = if let Some(off) = offset {
        let (_actual_offset, data) = file_store
            .read_at(&zone_id, &name, off, 0)
            .map_err(|e| format!("read_at: {}", e))?;
        data
    } else {
        file_store
            .read_file(&zone_id, &name)
            .map_err(|e| format!("read_file: {}", e))?
            .unwrap_or_default()
    };

    let data64 = base64::engine::general_purpose::STANDARD.encode(&data_bytes);
    let file_info_json = serde_json::to_value(&file_info)
        .map_err(|e| format!("serialize file_info: {}", e))?;

    Ok(serde_json::json!({
        "data": data64,
        "fileInfo": file_info_json,
    }))
}

/// Register an agent with the reactive messaging handler.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_register(
    block_id: String,
    agent_id: String,
    tab_id: Option<String>,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let handler = crate::backend::reactive::get_global_handler();

    // Set up the input sender if not already configured
    handler.set_input_sender(std::sync::Arc::new(|block_id: &str, data: &[u8]| {
        let input = crate::backend::blockcontroller::BlockInputUnion::data(data.to_vec());
        crate::backend::blockcontroller::send_input(block_id, input)
    }));

    handler
        .register_agent(&agent_id, &block_id, tab_id.as_deref())
        .map_err(|e| format!("register agent: {}", e))?;

    tracing::info!("reactive: registered agent {} -> block {}", agent_id, block_id);
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
    let resp_json = serde_json::to_value(&resp)
        .map_err(|e| format!("serialize response: {}", e))?;
    Ok(resp_json)
}

/// Configure the AgentBus poller for cross-host reactive messaging.
#[tauri::command(rename_all = "camelCase")]
pub async fn reactive_poller_config(
    agentbus_url: Option<String>,
    agentbus_token: Option<String>,
    _state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    // Validate URL if provided
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

/// Handle setconfig: merge values into settings.json, reload config, broadcast.
fn handle_set_config(data: &Value, state: &AppState) -> Result<Value, String> {
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

/// Handle setconnectionsconfig: merge values for a specific connection, reload, broadcast.
fn handle_set_connections_config(data: &Value, state: &AppState) -> Result<Value, String> {
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

// ---- Schema delivery via Tauri IPC ----

const SCHEMA_SETTINGS: &str = include_str!("../../../schema/settings.json");
const SCHEMA_CONNECTIONS: &str = include_str!("../../../schema/connections.json");
const SCHEMA_AIPRESETS: &str = include_str!("../../../schema/aipresets.json");
const SCHEMA_WIDGETS: &str = include_str!("../../../schema/widgets.json");

/// Tauri command to deliver JSON schema for Monaco editor validation.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_schema(schema_name: String) -> Result<Value, String> {
    let json_str = match schema_name.as_str() {
        "settings" => SCHEMA_SETTINGS,
        "connections" => SCHEMA_CONNECTIONS,
        "aipresets" => SCHEMA_AIPRESETS,
        "widgets" => SCHEMA_WIDGETS,
        _ => return Err(format!("unknown schema: {}", schema_name)),
    };
    serde_json::from_str(json_str)
        .map_err(|e| format!("parse schema {}: {}", schema_name, e))
}

/// Helper: get a WaveObj as JSON by otype/oid.
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
