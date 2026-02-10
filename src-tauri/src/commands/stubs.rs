// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Command handlers for IPC commands invoked by tauri-api.ts.
//
// Note: rename_all = "camelCase" ensures Rust snake_case params match
// the camelCase keys sent by TypeScript invoke() calls.

use serde_json::Value;

use crate::state::AppState;

// ---- Workspace commands ----

/// Create a new workspace with default name/icon/color.
#[tauri::command]
pub fn create_workspace(state: tauri::State<'_, AppState>) -> Result<(), String> {
    use crate::backend::wcore;

    let store = &state.wave_store;

    // Pick next color/icon based on workspace count
    let workspaces = wcore::list_workspaces(store)
        .map_err(|e| format!("failed to list workspaces: {}", e))?;
    let idx = workspaces.len();
    let color = wcore::WORKSPACE_COLORS[idx % wcore::WORKSPACE_COLORS.len()];
    let icon = wcore::WORKSPACE_ICONS[idx % wcore::WORKSPACE_ICONS.len()];

    let ws = wcore::create_workspace(store, "", icon, color)
        .map_err(|e| format!("failed to create workspace: {}", e))?;

    // Create an initial tab in the new workspace
    wcore::create_tab(store, &ws.oid)
        .map_err(|e| format!("failed to create tab: {}", e))?;

    tracing::info!("Created workspace: {}", &ws.oid[..8]);
    Ok(())
}

/// Switch the current window to a different workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn switch_workspace(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use crate::backend::wcore;

    let store = &state.wave_store;
    let window_id = state.window_id.lock().unwrap().clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    wcore::switch_workspace(store, &window_id, &workspace_id)
        .map_err(|e| format!("failed to switch workspace: {}", e))?;

    tracing::info!("Switched window {} to workspace {}", &window_id[..8.min(window_id.len())], &workspace_id[..8.min(workspace_id.len())]);
    Ok(())
}

/// Delete a workspace and all its tabs/blocks.
#[tauri::command(rename_all = "camelCase")]
pub fn delete_workspace(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    use crate::backend::wcore;

    let store = &state.wave_store;

    wcore::delete_workspace(store, &workspace_id)
        .map_err(|e| format!("failed to delete workspace: {}", e))?;

    tracing::info!("Deleted workspace: {}", &workspace_id[..8.min(workspace_id.len())]);
    Ok(())
}

// ---- Tab commands ----

/// Set the active tab in the current workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn set_active_tab(
    tab_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let store = &state.wave_store;
    let window_id = state.window_id.lock().unwrap().clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    // Get the window's current workspace
    let window = store.must_get::<crate::backend::waveobj::Window>(&window_id)
        .map_err(|e| format!("failed to get window: {}", e))?;

    crate::backend::wcore::set_active_tab(store, &window.workspaceid, &tab_id)
        .map_err(|e| format!("failed to set active tab: {}", e))?;

    // Update cached active tab ID
    *state.active_tab_id.lock().unwrap() = Some(tab_id.clone());

    tracing::debug!("Set active tab: {}", &tab_id[..8.min(tab_id.len())]);
    Ok(())
}

/// Create a new tab in the current workspace.
#[tauri::command]
pub fn create_tab(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let store = &state.wave_store;
    let window_id = state.window_id.lock().unwrap().clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    let window = store.must_get::<crate::backend::waveobj::Window>(&window_id)
        .map_err(|e| format!("failed to get window: {}", e))?;

    let tab = crate::backend::wcore::create_tab(store, &window.workspaceid)
        .map_err(|e| format!("failed to create tab: {}", e))?;

    tracing::info!("Created tab: {}", &tab.oid[..8]);
    Ok(())
}

/// Close a tab in a workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn close_tab(
    workspace_id: String,
    tab_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let store = &state.wave_store;

    crate::backend::wcore::delete_tab(store, &workspace_id, &tab_id)
        .map_err(|e| format!("failed to close tab: {}", e))?;

    // If the closed tab was the active one, update cache
    let mut active = state.active_tab_id.lock().unwrap();
    if active.as_deref() == Some(tab_id.as_str()) {
        let ws = store.must_get::<crate::backend::waveobj::Workspace>(&workspace_id)
            .map_err(|e| format!("failed to get workspace: {}", e))?;
        *active = if ws.activetabid.is_empty() { None } else { Some(ws.activetabid.clone()) };
    }

    tracing::info!("Closed tab: {}", &tab_id[..8.min(tab_id.len())]);
    Ok(())
}

// ---- Window init / state commands ----

/// Set the window initialization status.
#[tauri::command]
pub fn set_window_init_status(
    status: String,
    state: tauri::State<'_, AppState>,
) {
    tracing::debug!("set_window_init_status status={}", status);
    *state.window_init_status.lock().unwrap() = status;
}

/// Notify backend that the WaveAI panel is open/closed.
#[tauri::command(rename_all = "camelCase")]
pub fn set_waveai_open(is_open: bool) {
    tracing::debug!("set_waveai_open is_open={}", is_open);
}

// ---- Platform/UI commands ----

/// Trigger a file download via save dialog.
#[tauri::command]
pub async fn download_file(window: tauri::Window, path: String) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;

    let file_name = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");

    let dest = window
        .dialog()
        .file()
        .set_file_name(file_name)
        .blocking_save_file();

    let Some(dest) = dest else {
        return Ok(());
    }; // User cancelled

    let data =
        std::fs::read(&path).map_err(|e| format!("failed to read {}: {}", path, e))?;

    let dest_path = dest
        .as_path()
        .ok_or_else(|| "save dialog returned a non-filesystem path".to_string())?;

    std::fs::write(dest_path, &data)
        .map_err(|e| format!("failed to write: {}", e))?;

    Ok(())
}

/// Open macOS Quick Look preview for a file.
#[tauri::command(rename_all = "camelCase")]
pub fn quicklook(file_path: String) {
    #[cfg(target_os = "macos")]
    {
        std::thread::spawn(move || {
            let _ = std::process::Command::new("qlmanage")
                .arg("-p")
                .arg(&file_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        tracing::debug!("quicklook not available on this platform: {}", file_path);
    }
}

/// Update Window Controls Overlay rect.
/// Permanent no-op: WCO is Electron-specific. Tauri uses native window decorations.
#[tauri::command]
pub fn update_wco(_rect: Value) {}

/// Notify backend that keyboard chord mode is active.
#[tauri::command]
pub fn set_keyboard_chord_mode() {
    tracing::debug!("keyboard chord mode activated");
}

/// Register global webview keyboard shortcuts.
#[tauri::command]
pub fn register_global_webview_keys(keys: Vec<String>) {
    tracing::info!(
        "Registered {} global webview keys (Tauri native handling)",
        keys.len()
    );
}
