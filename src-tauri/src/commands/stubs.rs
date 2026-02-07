// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Stub command handlers for IPC commands invoked by tauri-api.ts
// that are not yet fully implemented. Each logs the call and returns Ok(()).
// These will be replaced with real implementations in later phases.

use serde_json::Value;

#[tauri::command]
pub fn show_context_menu(workspace_id: String, menu: Option<Value>) {
    tracing::debug!("stub: show_context_menu workspace={} menu={:?}", workspace_id, menu.is_some());
}

#[tauri::command]
pub fn download_file(path: String) {
    tracing::debug!("stub: download_file path={}", path);
}

#[tauri::command]
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

#[tauri::command]
pub fn switch_workspace(workspace_id: String) {
    tracing::debug!("stub: switch_workspace id={}", workspace_id);
}

#[tauri::command]
pub fn delete_workspace(workspace_id: String) {
    tracing::debug!("stub: delete_workspace id={}", workspace_id);
}

#[tauri::command]
pub fn set_active_tab(tab_id: String) {
    tracing::debug!("stub: set_active_tab id={}", tab_id);
}

#[tauri::command]
pub fn create_tab() {
    tracing::debug!("stub: create_tab");
}

#[tauri::command]
pub fn close_tab(workspace_id: String, tab_id: String) {
    tracing::debug!("stub: close_tab workspace={} tab={}", workspace_id, tab_id);
}

#[tauri::command]
pub fn set_window_init_status(status: String) {
    tracing::debug!("stub: set_window_init_status status={}", status);
}

#[tauri::command]
pub fn set_waveai_open(is_open: bool) {
    tracing::debug!("stub: set_waveai_open is_open={}", is_open);
}

#[tauri::command]
pub fn install_update() {
    tracing::debug!("stub: install_update");
}
