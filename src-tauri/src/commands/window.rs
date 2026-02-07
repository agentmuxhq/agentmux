use tauri::Emitter;
use tauri::Manager;

use crate::state::AppState;

/// Open a new WaveMux window.
/// Replaces: ipcMain.on("open-new-window") in emain/emain.ts
#[tauri::command]
pub async fn open_new_window(app: tauri::AppHandle) -> Result<(), String> {
    let label = format!("window-{}", uuid::Uuid::new_v4().simple());

    tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("WaveMux")
    .inner_size(1200.0, 800.0)
    .min_inner_size(400.0, 300.0)
    .decorations(false)
    .transparent(true)
    .build()
    .map_err(|e| format!("Failed to create window: {}", e))?;

    Ok(())
}

/// Get the current zoom factor.
/// Replaces: ipcMain.on("get-zoom-factor") in emain/emain.ts
#[tauri::command]
pub fn get_zoom_factor(state: tauri::State<'_, AppState>) -> f64 {
    *state.zoom_factor.lock().unwrap()
}

/// Set the zoom factor.
/// Replaces: webContents.setZoomFactor() calls in emain/menu.ts
#[tauri::command]
pub fn set_zoom_factor(
    state: tauri::State<'_, AppState>,
    window: tauri::Window,
    factor: f64,
) -> Result<(), String> {
    let factor = factor.clamp(0.5, 3.0);
    *state.zoom_factor.lock().unwrap() = factor;

    // Tauri uses webview.zoom() for zoom factor
    if let Some(webview) = window.app_handle().get_webview_window(&window.label()) {
        webview
            .set_zoom(factor)
            .map_err(|e| format!("Failed to set zoom: {}", e))?;
    }

    // Notify frontend of zoom change
    let _ = window.emit("zoom-factor-change", factor);

    Ok(())
}

/// Get the cursor position relative to the screen.
/// Replaces: ipcMain.on("get-cursor-point") in emain/emain.ts
#[tauri::command]
pub fn get_cursor_point(window: tauri::Window) -> Result<serde_json::Value, String> {
    let position = window
        .cursor_position()
        .map_err(|e| format!("Failed to get cursor position: {}", e))?;

    Ok(serde_json::json!({
        "x": position.x,
        "y": position.y,
    }))
}
