// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Stub command handlers for IPC commands invoked by tauri-api.ts
// that are not yet fully implemented. Each logs the call and returns Ok(()).
// These will be replaced with real implementations in later phases.
//
// Note: rename_all = "camelCase" ensures Rust snake_case params match
// the camelCase keys sent by TypeScript invoke() calls.

use serde_json::Value;
use tauri::{Emitter, Manager};

#[tauri::command(rename_all = "camelCase")]
pub fn show_context_menu(workspace_id: String, menu: Option<Value>) {
    tracing::debug!("stub: show_context_menu workspace={} menu={:?}", workspace_id, menu.is_some());
}

#[tauri::command]
pub fn download_file(path: String) {
    tracing::debug!("stub: download_file path={}", path);
}

#[tauri::command(rename_all = "camelCase")]
pub fn quicklook(file_path: String) {
    tracing::debug!("stub: quicklook path={}", file_path);
}

#[tauri::command]
pub fn update_wco(rect: Value) {
    tracing::debug!("stub: update_wco rect={}", rect);
}

#[tauri::command]
pub fn set_keyboard_chord_mode() {
    tracing::debug!("stub: set_keyboard_chord_mode");
}

#[tauri::command]
pub fn register_global_webview_keys(keys: Vec<String>) {
    tracing::debug!("stub: register_global_webview_keys keys={:?}", keys);
}

#[tauri::command]
pub fn create_workspace() {
    tracing::debug!("stub: create_workspace");
}

#[tauri::command(rename_all = "camelCase")]
pub fn switch_workspace(workspace_id: String) {
    tracing::debug!("stub: switch_workspace id={}", workspace_id);
}

#[tauri::command(rename_all = "camelCase")]
pub fn delete_workspace(workspace_id: String) {
    tracing::debug!("stub: delete_workspace id={}", workspace_id);
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_active_tab(tab_id: String) {
    tracing::debug!("stub: set_active_tab id={}", tab_id);
}

#[tauri::command]
pub fn create_tab() {
    tracing::debug!("stub: create_tab");
}

#[tauri::command(rename_all = "camelCase")]
pub fn close_tab(workspace_id: String, tab_id: String) {
    tracing::debug!("stub: close_tab workspace={} tab={}", workspace_id, tab_id);
}

#[tauri::command]
pub async fn set_window_init_status(
    status: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<(), String> {
    tracing::debug!("set_window_init_status status={}", status);

    // Store the status
    *state.window_init_status.lock().unwrap() = status.clone();

    // When status is "ready", fetch client data from backend and emit wave-init
    if status == "ready" {
        tracing::info!("Fetching client data from backend");

        // Get backend endpoint and auth key
        let (web_endpoint, auth_key) = {
            let endpoints = state.backend_endpoints.lock().unwrap();
            let auth = state.auth_key.lock().unwrap();
            (endpoints.web_endpoint.clone(), auth.clone())
        };

        if web_endpoint.is_empty() {
            return Err("Backend not ready".to_string());
        }

        // Call GetClientData with proper RPC format
        let url = format!("http://{}/wave/service?service=client&method=GetClientData&authkey={}",
            web_endpoint, auth_key);

        let rpc_body = serde_json::json!({
            "service": "client",
            "method": "GetClientData",
            "args": [],
            "uicontext": null
        });

        let client = reqwest::Client::new();
        let response = client.post(&url)
            .json(&rpc_body)
            .send()
            .await
            .map_err(|e| format!("GetClientData request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("GetClientData failed: {}", response.status()));
        }

        let client_data: serde_json::Value = response.json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        tracing::info!("GetClientData response: {}", serde_json::to_string_pretty(&client_data).unwrap_or_default());

        // Extract from nested "data" field
        let data = client_data.get("data").unwrap_or(&client_data);

        let client_id = data.get("oid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let window_id = data.get("windowids")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        tracing::info!("Got client data: clientId={}, windowId={}", client_id, window_id);

        // Get window object to extract workspace and tab
        let mut tab_id = String::new();
        if !window_id.is_empty() {
            let window_url = format!("http://{}/wave/service?service=object&method=GetObject&authkey={}",
                web_endpoint, auth_key);
            let window_rpc = serde_json::json!({
                "service": "object",
                "method": "GetObject",
                "args": [window_id],
                "uicontext": null
            });

            if let Ok(resp) = client.post(&window_url).json(&window_rpc).send().await {
                if let Ok(window_data) = resp.json::<serde_json::Value>().await {
                    tracing::info!("Window data: {}", serde_json::to_string_pretty(&window_data).unwrap_or_default());

                    if let Some(data) = window_data.get("data") {
                        // Get workspace ID from window
                        if let Some(workspace_id) = data.get("workspaceid").and_then(|v| v.as_str()) {
                            // Get workspace to find active tab
                            let workspace_rpc = serde_json::json!({
                                "service": "object",
                                "method": "GetObject",
                                "args": [workspace_id],
                                "uicontext": null
                            });

                            if let Ok(ws_resp) = client.post(&window_url).json(&workspace_rpc).send().await {
                                if let Ok(ws_data) = ws_resp.json::<serde_json::Value>().await {
                                    if let Some(ws) = ws_data.get("data") {
                                        tab_id = ws.get("tabids")
                                            .and_then(|v| v.as_array())
                                            .and_then(|arr| arr.first())
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Emitting wave-init: clientId={}, windowId={}, tabId={}",
            client_id, window_id, tab_id);

        // Emit wave-init with actual IDs
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("wave-init", serde_json::json!({
                "clientId": client_id,
                "windowId": window_id,
                "tabId": tab_id,
                "activate": true,
                "primaryTabStartup": true,
            }));
        }
    }

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_waveai_open(is_open: bool) {
    tracing::debug!("stub: set_waveai_open is_open={}", is_open);
}

#[tauri::command]
pub fn install_update() {
    tracing::debug!("stub: install_update");
}
