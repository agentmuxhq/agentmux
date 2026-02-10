// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Tab Tauri commands — create, activate, close tabs.

use crate::state::AppState;

/// Set the active tab in the current workspace.
#[tauri::command(rename_all = "camelCase")]
pub fn set_active_tab(
    tab_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let store = &state.wave_store;
    let window_id = state
        .window_id
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    let window = store
        .must_get::<crate::backend::waveobj::Window>(&window_id)
        .map_err(|e| format!("failed to get window: {}", e))?;

    crate::backend::wcore::set_active_tab(store, &window.workspaceid, &tab_id)
        .map_err(|e| format!("failed to set active tab: {}", e))?;

    *state.active_tab_id.lock().unwrap() = Some(tab_id.clone());

    tracing::debug!("Set active tab: {}", &tab_id[..8.min(tab_id.len())]);
    Ok(())
}

/// Create a new tab in the current workspace.
#[tauri::command]
pub fn create_tab(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let store = &state.wave_store;
    let window_id = state
        .window_id
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    let window = store
        .must_get::<crate::backend::waveobj::Window>(&window_id)
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
        let ws = store
            .must_get::<crate::backend::waveobj::Workspace>(&workspace_id)
            .map_err(|e| format!("failed to get workspace: {}", e))?;
        *active = if ws.activetabid.is_empty() {
            None
        } else {
            Some(ws.activetabid.clone())
        };
    }

    tracing::info!("Closed tab: {}", &tab_id[..8.min(tab_id.len())]);
    Ok(())
}
