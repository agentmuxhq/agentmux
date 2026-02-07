use crate::state::AppState;

/// Get the backend WebSocket and HTTP endpoints.
///
/// The frontend uses these to establish its WebSocket RPC connection
/// and make HTTP requests to the Go backend.
///
/// This is a new command (no Electron equivalent) -- in Electron,
/// the frontend received these via the wave-init event pushed
/// from the main process.
#[tauri::command]
pub fn get_backend_endpoints(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let endpoints = state.backend_endpoints.lock().unwrap();

    if endpoints.ws_endpoint.is_empty() {
        return Err("Backend not ready yet".to_string());
    }

    Ok(serde_json::json!({
        "ws": endpoints.ws_endpoint,
        "web": endpoints.web_endpoint,
    }))
}

/// Log a message from the frontend.
/// Replaces: ipcMain.on("fe-log") in emain/emain.ts
#[tauri::command]
pub fn fe_log(msg: String) {
    tracing::info!("[frontend] {}", msg);
}
