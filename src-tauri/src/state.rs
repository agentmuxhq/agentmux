use std::sync::Mutex;

/// Shared application state managed by Tauri.
/// Replaces the scattered state across emain/*.ts files.
pub struct AppState {
    /// Auth key for backend communication (replaces emain/authkey.ts)
    pub auth_key: String,

    /// Backend (wavemuxsrv) connection endpoints
    pub backend_endpoints: Mutex<BackendEndpoints>,

    /// Handle to the sidecar child process for graceful shutdown
    pub sidecar_child: Mutex<Option<tauri_plugin_shell::process::CommandChild>>,

    /// Current zoom factor (replaces Electron's webContents zoom)
    pub zoom_factor: Mutex<f64>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct BackendEndpoints {
    pub ws_endpoint: String,
    pub web_endpoint: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth_key: uuid::Uuid::new_v4().to_string(),
            backend_endpoints: Mutex::new(BackendEndpoints::default()),
            sidecar_child: Mutex::new(None),
            zoom_factor: Mutex::new(1.0),
        }
    }
}
