use std::sync::Arc;
use std::sync::Mutex;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// Auth key for backend communication
    pub auth_key: Mutex<String>,

    /// Current zoom factor
    pub zoom_factor: Mutex<f64>,

    /// Client ID
    pub client_id: Mutex<Option<String>>,

    /// Window ID
    pub window_id: Mutex<Option<String>>,

    /// Active tab ID
    pub active_tab_id: Mutex<Option<String>>,

    /// Window initialization status ("ready" or "wave-ready")
    pub window_init_status: Mutex<String>,

    /// Rust-native backend state
    pub wave_store: Arc<crate::backend::storage::wstore::WaveStore>,

    pub broker: Arc<crate::backend::wps::Broker>,

    pub rpc_engine: Arc<crate::backend::rpc::engine::WshRpcEngine>,

    pub router: Arc<crate::backend::rpc::router::WshRouter>,

    pub file_store: Arc<crate::backend::storage::filestore::FileStore>,

    /// Path to the wsh IPC socket (named pipe on Windows, Unix socket on macOS/Linux)
    pub wsh_socket_path: Mutex<String>,

    /// Config watcher holding the full loaded config
    pub config_watcher: Arc<crate::backend::wconfig::ConfigWatcher>,

    /// Path to the Wave config directory (~/.waveterm/config)
    pub config_dir: std::path::PathBuf,
}
