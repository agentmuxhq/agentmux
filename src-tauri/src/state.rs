use std::sync::Mutex;

#[cfg(feature = "rust-backend")]
use std::sync::Arc;

/// Shared application state managed by Tauri.
/// Replaces the scattered state across emain/*.ts files.
pub struct AppState {
    /// Auth key for backend communication (replaces emain/authkey.ts)
    /// Can be updated after querying database for existing client
    pub auth_key: Mutex<String>,

    /// Backend (agentmuxsrv) connection endpoints (Go sidecar mode only)
    #[cfg(feature = "go-sidecar")]
    pub backend_endpoints: Mutex<BackendEndpoints>,

    /// Handle to the sidecar child process for graceful shutdown (Go sidecar mode only)
    #[cfg(feature = "go-sidecar")]
    pub sidecar_child: Mutex<Option<tauri_plugin_shell::process::CommandChild>>,

    /// Current zoom factor (replaces Electron's webContents zoom)
    pub zoom_factor: Mutex<f64>,

    /// Client ID (replaces Electron's clientData tracking)
    pub client_id: Mutex<Option<String>>,

    /// Window ID (replaces Electron's window tracking)
    pub window_id: Mutex<Option<String>>,

    /// Active tab ID (replaces Electron's tab tracking)
    pub active_tab_id: Mutex<Option<String>>,

    /// Window initialization status ("ready" or "wave-ready")
    pub window_init_status: Mutex<String>,

    /// Rust-native backend state (rust-backend mode only)
    #[cfg(feature = "rust-backend")]
    pub wave_store: Arc<crate::backend::storage::wstore::WaveStore>,

    #[cfg(feature = "rust-backend")]
    pub broker: Arc<crate::backend::wps::Broker>,

    #[cfg(feature = "rust-backend")]
    pub rpc_engine: Arc<crate::backend::rpc::engine::WshRpcEngine>,

    #[cfg(feature = "rust-backend")]
    pub router: Arc<crate::backend::rpc::router::WshRouter>,

    #[cfg(feature = "rust-backend")]
    pub file_store: Arc<crate::backend::storage::filestore::FileStore>,

    /// Path to the wsh IPC socket (named pipe on Windows, Unix socket on macOS/Linux)
    #[cfg(feature = "rust-backend")]
    pub wsh_socket_path: Mutex<String>,

    /// Config watcher holding the full loaded config (rust-backend mode only)
    #[cfg(feature = "rust-backend")]
    pub config_watcher: Arc<crate::backend::wconfig::ConfigWatcher>,

    /// Path to the Wave config directory (~/.waveterm/config)
    #[cfg(feature = "rust-backend")]
    pub config_dir: std::path::PathBuf,
}

#[derive(Default, Clone, serde::Serialize)]
pub struct BackendEndpoints {
    pub ws_endpoint: String,
    pub web_endpoint: String,
}

#[cfg(feature = "go-sidecar")]
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
