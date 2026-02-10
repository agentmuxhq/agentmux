// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Workspace Tauri commands — create, switch, delete workspaces.

use crate::state::AppState;

/// Create a new workspace with default name/icon/color.
#[tauri::command]
pub fn create_workspace(state: tauri::State<'_, AppState>) -> Result<(), String> {
    use crate::backend::wcore;

    let store = &state.wave_store;

    let workspaces =
        wcore::list_workspaces(store).map_err(|e| format!("failed to list workspaces: {}", e))?;
    let idx = workspaces.len();
    let color = wcore::WORKSPACE_COLORS[idx % wcore::WORKSPACE_COLORS.len()];
    let icon = wcore::WORKSPACE_ICONS[idx % wcore::WORKSPACE_ICONS.len()];

    let ws = wcore::create_workspace(store, "", icon, color)
        .map_err(|e| format!("failed to create workspace: {}", e))?;

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
    let window_id = state
        .window_id
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "window not initialized".to_string())?;

    wcore::switch_workspace(store, &window_id, &workspace_id)
        .map_err(|e| format!("failed to switch workspace: {}", e))?;

    tracing::info!(
        "Switched window {} to workspace {}",
        &window_id[..8.min(window_id.len())],
        &workspace_id[..8.min(workspace_id.len())]
    );
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

    tracing::info!(
        "Deleted workspace: {}",
        &workspace_id[..8.min(workspace_id.len())]
    );
    Ok(())
}
