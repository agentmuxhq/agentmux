// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Command handlers for IPC commands invoked by tauri-api.ts.
//
// In go-sidecar mode: workspace/tab commands are stubs (Go handles them via WebSocket RPC).
// In rust-backend mode: workspace/tab commands use WaveStore directly.
// Platform commands (context menu, download, quicklook, etc.) remain stubs for now.
//
// Note: rename_all = "camelCase" ensures Rust snake_case params match
// the camelCase keys sent by TypeScript invoke() calls.

use serde_json::Value;

use crate::state::AppState;

// ---- Workspace commands ----

/// Create a new workspace with default name/icon/color.
/// In rust-backend mode: creates in WaveStore and updates window.
/// In go-sidecar mode: stub (Go backend handles via WebSocket RPC).
#[tauri::command]
pub fn create_workspace(state: tauri::State<'_, AppState>) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
        use crate::backend::wcore;

        let store = &state.wave_store;
        let window_id = state.window_id.lock().unwrap().clone()
            .ok_or_else(|| "window not initialized".to_string())?;

        // Get current window to find its workspace
        let window = store.must_get::<crate::backend::waveobj::Window>(&window_id)
            .map_err(|e| format!("failed to get window: {}", e))?;

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
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: create_workspace");
        Ok(())
    }
}

/// Switch the current window to a different workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn switch_workspace(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
        use crate::backend::wcore;

        let store = &state.wave_store;
        let window_id = state.window_id.lock().unwrap().clone()
            .ok_or_else(|| "window not initialized".to_string())?;

        wcore::switch_workspace(store, &window_id, &workspace_id)
            .map_err(|e| format!("failed to switch workspace: {}", e))?;

        tracing::info!("Switched window {} to workspace {}", &window_id[..8], &workspace_id[..8]);
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: switch_workspace id={}", workspace_id);
        Ok(())
    }
}

/// Delete a workspace and all its tabs/blocks.
#[tauri::command(rename_all = "camelCase")]
pub fn delete_workspace(
    workspace_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
        use crate::backend::wcore;

        let store = &state.wave_store;

        wcore::delete_workspace(store, &workspace_id)
            .map_err(|e| format!("failed to delete workspace: {}", e))?;

        tracing::info!("Deleted workspace: {}", &workspace_id[..8]);
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: delete_workspace id={}", workspace_id);
        Ok(())
    }
}

// ---- Tab commands ----

/// Set the active tab in the current workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn set_active_tab(
    tab_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
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
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: set_active_tab id={}", tab_id);
        Ok(())
    }
}

/// Create a new tab in the current workspace.
#[tauri::command]
pub fn create_tab(state: tauri::State<'_, AppState>) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
        let store = &state.wave_store;
        let window_id = state.window_id.lock().unwrap().clone()
            .ok_or_else(|| "window not initialized".to_string())?;

        let window = store.must_get::<crate::backend::waveobj::Window>(&window_id)
            .map_err(|e| format!("failed to get window: {}", e))?;

        let tab = crate::backend::wcore::create_tab(store, &window.workspaceid)
            .map_err(|e| format!("failed to create tab: {}", e))?;

        tracing::info!("Created tab: {}", &tab.oid[..8]);
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: create_tab");
        Ok(())
    }
}

/// Close a tab in a workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn close_tab(
    workspace_id: String,
    tab_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    #[cfg(feature = "rust-backend")]
    {
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
        return Ok(());
    }

    #[cfg(not(feature = "rust-backend"))]
    {
        let _ = state;
        tracing::debug!("stub: close_tab workspace={} tab={}", workspace_id, tab_id);
        Ok(())
    }
}

// ---- Window init / state commands ----

/// Set the window initialization status.
/// Already functional in both modes — just stores the status string.
#[tauri::command]
pub fn set_window_init_status(
    status: String,
    state: tauri::State<'_, AppState>,
) {
    tracing::debug!("set_window_init_status status={}", status);
    *state.window_init_status.lock().unwrap() = status;
}

/// Notify backend that the WaveAI panel is open/closed.
/// In rust-backend mode: could be used to manage AI resources.
/// Currently just logs the state.
#[tauri::command(rename_all = "camelCase")]
pub fn set_waveai_open(is_open: bool) {
    tracing::debug!("set_waveai_open is_open={}", is_open);
}

// ---- Platform/UI stubs (to be implemented in later phases) ----

/// Show a native context menu.
/// TODO: Phase C — implement with tauri::menu::Menu
#[tauri::command(rename_all = "camelCase")]
pub fn show_context_menu(workspace_id: String, menu: Option<Value>) {
    tracing::debug!("stub: show_context_menu workspace={} menu={:?}", workspace_id, menu.is_some());
}

/// Trigger a file download.
/// TODO: Phase F — implement with tauri_plugin_dialog save_file
#[tauri::command]
pub fn download_file(path: String) {
    tracing::debug!("stub: download_file path={}", path);
}

/// Open macOS Quick Look preview for a file.
/// TODO: macOS only — implement with NSWorkspace API
#[tauri::command(rename_all = "camelCase")]
pub fn quicklook(file_path: String) {
    tracing::debug!("stub: quicklook path={}", file_path);
}

/// Update Window Controls Overlay rect.
/// Not needed in Tauri (uses native decorations or custom titlebar).
#[tauri::command]
pub fn update_wco(rect: Value) {
    tracing::debug!("stub: update_wco rect={}", rect);
}

/// Set keyboard chord mode for multi-key shortcuts.
/// TODO: implement with tauri_plugin_global_shortcut
#[tauri::command]
pub fn set_keyboard_chord_mode() {
    tracing::debug!("stub: set_keyboard_chord_mode");
}

/// Register global webview keyboard shortcuts.
/// TODO: implement with tauri_plugin_global_shortcut
#[tauri::command]
pub fn register_global_webview_keys(keys: Vec<String>) {
    tracing::debug!("stub: register_global_webview_keys keys={:?}", keys);
}

/// Install a pending app update.
/// TODO: implement with tauri_plugin_updater
#[tauri::command]
pub fn install_update() {
    tracing::debug!("stub: install_update");
}
