use crate::state::AppState;

/// Get the backend WebSocket and HTTP endpoints.
///
/// In go-sidecar mode: returns the Go backend's WS/HTTP endpoints.
/// In rust-backend mode: returns empty endpoints (frontend uses Tauri IPC).
#[tauri::command]
pub fn get_backend_endpoints(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    #[cfg(feature = "go-sidecar")]
    {
        let endpoints = state.backend_endpoints.lock().unwrap();
        if endpoints.ws_endpoint.is_empty() {
            return Err("Backend not ready yet".to_string());
        }
        return Ok(serde_json::json!({
            "ws": endpoints.ws_endpoint,
            "web": endpoints.web_endpoint,
        }));
    }

    #[cfg(feature = "rust-backend")]
    {
        // In rust-backend mode, no WebSocket/HTTP — frontend uses Tauri IPC
        let _ = state; // suppress unused warning
        Ok(serde_json::json!({
            "ws": "",
            "web": "",
            "rustBackend": true,
        }))
    }

    #[cfg(not(any(feature = "go-sidecar", feature = "rust-backend")))]
    {
        let _ = state;
        Err("No backend feature enabled".to_string())
    }
}

/// Get the window initialization options (client/window/tab IDs).
/// Used by the frontend to initialize the Wave application.
#[tauri::command]
pub fn get_wave_init_opts(
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client_id = state.client_id.lock().unwrap();
    let window_id = state.window_id.lock().unwrap();
    let tab_id = state.active_tab_id.lock().unwrap();

    if client_id.is_none() || window_id.is_none() || tab_id.is_none() {
        return Err("Window state not initialized yet".to_string());
    }

    Ok(serde_json::json!({
        "clientId": client_id.as_ref().unwrap(),
        "windowId": window_id.as_ref().unwrap(),
        "tabId": tab_id.as_ref().unwrap(),
        "activate": true,
        "primaryTabStartup": true,
    }))
}

/// Log a message from the frontend.
/// Replaces: ipcMain.on("fe-log") in emain/emain.ts
#[tauri::command]
pub fn fe_log(msg: String) {
    tracing::info!("[frontend] {}", msg);
}
