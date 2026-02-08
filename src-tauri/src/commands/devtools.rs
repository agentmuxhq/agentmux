use tauri::Manager;

/// Toggle devtools for the current window.
/// Maps to Ctrl+Shift+I / Cmd+Option+I keyboard shortcut.
#[tauri::command]
pub fn toggle_devtools(window: tauri::Window) {
    #[cfg(debug_assertions)]
    {
        if window.is_devtools_open() {
            tracing::info!("Closing devtools for window: {}", window.label());
            window.close_devtools();
        } else {
            tracing::info!("Opening devtools for window: {}", window.label());
            window.open_devtools();
        }
    }

    #[cfg(not(debug_assertions))]
    {
        tracing::warn!(
            "Devtools toggle requested in release build for window: {}",
            window.label()
        );
        // Optionally allow in release if needed
        if window.is_devtools_open() {
            window.close_devtools();
        } else {
            window.open_devtools();
        }
    }
}

/// Check if devtools are currently open.
#[tauri::command]
pub fn is_devtools_open(window: tauri::Window) -> bool {
    window.is_devtools_open()
}
