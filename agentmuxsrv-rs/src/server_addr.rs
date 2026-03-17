use std::sync::OnceLock;

static BACKEND_WEB_ADDR: OnceLock<String> = OnceLock::new();

/// Called once from main.rs after the web listener binds.
/// Subsequent calls are silently ignored (OnceLock semantics).
pub fn set(addr: &str) {
    let _ = BACKEND_WEB_ADDR.set(addr.to_string());
}

/// Returns `http://127.0.0.1:{port}` or None if not yet set.
pub fn local_url() -> Option<String> {
    BACKEND_WEB_ADDR.get().map(|addr| format!("http://{}", addr))
}
