use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;

use crate::state::AppState;

/// Open a new AgentMux window.
/// Replaces: ipcMain.on("open-new-window") in emain/emain.ts
///
/// Creates a new window that will initialize with a new backend Window/Workspace/Tab.
/// The frontend detects it's a new window by checking if it's NOT the "main" window,
/// and triggers backend object creation via initTauriWave().
#[tauri::command]
pub async fn open_new_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    let window_id = uuid::Uuid::new_v4();
    let label = format!("window-{}", window_id.simple());
    let version = env!("CARGO_PKG_VERSION");
    let title = format!("AgentMux {}", version);

    tauri::WebviewWindowBuilder::new(
        &app,
        &label,
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title(&title)
    .inner_size(1200.0, 800.0)
    .min_inner_size(400.0, 300.0)
    .decorations(false)
    .visible(false) // Start hidden, show after initialization
    .build()
    .map_err(|e| format!("Failed to create window: {}", e))?;

    tracing::info!("Created new window with label: {}", label);
    Ok(label)
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
pub fn set_zoom_factor<R: Runtime>(
    state: tauri::State<'_, AppState>,
    window: tauri::WebviewWindow<R>,
    factor: f64,
) -> Result<(), String> {
    let factor = factor.clamp(0.5, 3.0);
    *state.zoom_factor.lock().unwrap() = factor;

    // Tauri uses webview.set_zoom() for zoom factor
    window
        .set_zoom(factor)
        .map_err(|e| format!("Failed to set zoom: {}", e))?;

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

/// Close a specific window by label.
/// If no label provided, closes the calling window.
#[tauri::command]
pub fn close_window(
    app: tauri::AppHandle,
    window: tauri::Window,
    label: Option<String>,
) -> Result<(), String> {
    let target_label = label.unwrap_or_else(|| window.label().to_string());

    if let Some(target_window) = app.get_webview_window(&target_label) {
        tracing::info!("Closing window: {}", target_label);
        target_window
            .close()
            .map_err(|e| format!("Failed to close window: {}", e))?;
    } else {
        return Err(format!("Window not found: {}", target_label));
    }

    Ok(())
}

/// Get the current window label.
#[tauri::command]
pub fn get_window_label(window: tauri::Window) -> String {
    window.label().to_string()
}

/// Check if this is the main window.
#[tauri::command]
pub fn is_main_window(window: tauri::Window) -> bool {
    window.label() == "main"
}

/// List all open window labels.
#[tauri::command]
pub fn list_windows(app: tauri::AppHandle) -> Vec<String> {
    app.webview_windows()
        .keys()
        .map(|k| k.to_string())
        .collect()
}

/// Focus a specific window by label.
#[tauri::command]
pub fn focus_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(&label) {
        window
            .set_focus()
            .map_err(|e| format!("Failed to focus window: {}", e))?;
        tracing::info!("Focused window: {}", label);
    } else {
        return Err(format!("Window not found: {}", label));
    }
    Ok(())
}
