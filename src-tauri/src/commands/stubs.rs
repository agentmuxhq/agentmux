// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Miscellaneous commands — platform utilities and permanent no-ops.

use serde_json::Value;

use crate::state::AppState;

// ---- Window init / state commands ----

/// Set the window initialization status.
#[tauri::command]
pub fn set_window_init_status(status: String, state: tauri::State<'_, AppState>) {
    tracing::debug!("set_window_init_status status={}", status);
    *state.window_init_status.lock().unwrap() = status;
}

/// Notify backend that the WaveAI panel is open/closed.
#[tauri::command(rename_all = "camelCase")]
pub fn set_waveai_open(is_open: bool) {
    tracing::debug!("set_waveai_open is_open={}", is_open);
}

// ---- File operations ----

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
    };

    let data = std::fs::read(&path).map_err(|e| format!("failed to read {}: {}", path, e))?;

    let dest_path = dest
        .as_path()
        .ok_or_else(|| "save dialog returned a non-filesystem path".to_string())?;

    std::fs::write(dest_path, &data).map_err(|e| format!("failed to write: {}", e))?;

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

// ---- Permanent no-ops ----

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
