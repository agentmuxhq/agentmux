use std::sync::Mutex;

/// Shared application state managed by Tauri.
/// Replaces the scattered state across emain/*.ts files.
pub struct AppState {
    /// Auth key for backend communication (replaces emain/authkey.ts)
    /// Can be updated after querying database for existing client
    pub auth_key: Mutex<String>,

    /// Backend (agentmuxsrv-rs) connection endpoints
    pub backend_endpoints: Mutex<BackendEndpoints>,

    /// Handle to the sidecar child process for graceful shutdown
    pub sidecar_child: Mutex<Option<tauri_plugin_shell::process::CommandChild>>,

    /// Current zoom factor (replaces Electron's webContents zoom)
    pub zoom_factor: Mutex<f64>,

    /// Client ID (replaces Electron's clientData tracking)
    /// Set after querying backend on startup
    pub client_id: Mutex<Option<String>>,

    /// Window ID (replaces Electron's window tracking)
    /// Set after querying/creating window via backend
    pub window_id: Mutex<Option<String>>,

    /// Active tab ID (replaces Electron's tab tracking)
    /// Set after querying/creating default tab via backend
    pub active_tab_id: Mutex<Option<String>>,

    /// Window initialization status ("ready" or "wave-ready")
    pub window_init_status: Mutex<String>,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct BackendEndpoints {
    pub ws_endpoint: String,
    pub web_endpoint: String,
    pub is_reused: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth_key: Mutex::new(uuid::Uuid::new_v4().to_string()),
            backend_endpoints: Mutex::new(BackendEndpoints::default()),
            sidecar_child: Mutex::new(None),
            zoom_factor: Mutex::new(1.0),
            client_id: Mutex::new(None),
            window_id: Mutex::new(None),
            active_tab_id: Mutex::new(None),
            window_init_status: Mutex::new(String::new()),
        }
    }
}
