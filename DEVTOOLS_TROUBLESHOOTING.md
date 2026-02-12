# DevTools Button Troubleshooting Guide

## Quick Checklist

1. **Check Browser Console** (F12 in app)
   - Look for: `[tauri-api] Calling toggle_devtools command`
   - Look for: `[tauri-api] toggle_devtools succeeded` OR error

2. **Check Rust Logs**
   - Windows: `%APPDATA%\com.a5af.wavemux\logs\waveapp.log`
   - Look for: "Opening devtools for window" or "Closing devtools for window"

3. **Test Menu Shortcut**
   - Try: Alt+Shift+I (Windows) or Option+Command+I (Mac)
   - If this works but button doesn't → frontend invoke issue
   - If this also doesn't work → backend issue

## Debugging Steps

### Step 1: Verify Frontend Call
Open DevTools (if possible via menu) and click the DevTools button in WidgetBar.

**Expected console output:**
```
[tauri-api] Calling toggle_devtools command
[tauri-api] toggle_devtools succeeded
```

**If you see nothing:**
- Button click handler not firing
- Check: `frontend/app/tab/widgetbar.tsx:29`

**If you see error:**
- Check error message
- See "Common Errors" below

### Step 2: Verify Backend Receives Call
Check Rust logs after clicking button.

**Expected log:**
```
Opening devtools for window: main
```
OR
```
Closing devtools for window: main
```

**If you see nothing in logs:**
- Command not registered (check lib.rs:72)
- Invoke failing silently (check capabilities)

### Step 3: Test Direct Rust Call
Modify `src-tauri/src/commands/devtools.rs` temporarily:

```rust
#[tauri::command]
pub fn toggle_devtools(window: tauri::WebviewWindow) {
    tracing::info!("toggle_devtools CALLED - window label: {}", window.label());
    // ... rest of code
}
```

Rebuild and test. If you see this log, command is being called.

## Common Errors

### Error: "Command toggle_devtools not found"
**Cause**: Command not in invoke_handler
**Fix**: Verify `lib.rs:72` has `commands::devtools::toggle_devtools,`

### Error: "Permission denied"
**Cause**: Missing capability permission
**Fix**: Add to `capabilities/default.json`:
```json
"core:webview:allow-internal-toggle-devtools"
```
(Should be included in `core:webview:default`)

### Error: "Window not found"
**Cause**: invoke() doesn't have window context
**Fix**: Try calling from window context or pass label explicitly

### No Error, But Nothing Happens
**Possible causes:**
1. Devtools already in desired state
2. Feature not enabled in Cargo.toml
3. Webview vs Window object confusion
4. Silent failure in open_devtools/close_devtools

**Debug:**
```rust
if window.is_devtools_open() {
    tracing::info!("Devtools already open, closing...");
    match window.close_devtools() {
        Ok(_) => tracing::info!("Successfully closed devtools"),
        Err(e) => tracing::error!("Failed to close devtools: {:?}", e),
    }
}
```

## Comparison with Working Menu Integration

The menu system calls the same window methods:

**Menu Handler** (`menu.rs:227-236`):
```rust
"toggle-devtools" => {
    if let Some(w) = window {
        if w.is_devtools_open() {
            tracing::info!("Closing devtools for window: {}", w.label());
            let _ = w.close_devtools();
        } else {
            tracing::info!("Opening devtools for window: {}", w.label());
            let _ = w.open_devtools();
        }
    }
}
```

**Our Command** (`devtools.rs:4-27`):
```rust
#[tauri::command]
pub fn toggle_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}
```

Both call the same methods. Difference:
- Menu gets window from event context
- Command gets window from Tauri's auto-injection

## Alternative Implementations

### Option 1: Return Result
```typescript
toggleDevtools: async () => {
    try {
        console.log("[tauri-api] Calling toggle_devtools");
        await invoke("toggle_devtools");
        console.log("[tauri-api] DevTools toggled successfully");
    } catch (err) {
        console.error("[tauri-api] Failed to toggle devtools:", err);
        throw err;
    }
}
```

### Option 2: With Window Label
```typescript
toggleDevtools: async () => {
    const label = await invoke<string>("get_window_label");
    await invoke("toggle_devtools", { window: label });
}
```

### Option 3: Call Menu Command
```typescript
toggleDevtools: () => {
    // Trigger the menu item programmatically
    // (if Tauri supports this)
}
```

## Verification After Fix

1. Click DevTools button
2. DevTools panel should appear
3. Click again
4. DevTools panel should disappear
5. Check logs show toggle messages
6. Verify works in both dev and release builds

## Related Files

- `frontend/app/tab/widgetbar.tsx` - Button component
- `frontend/util/tauri-api.ts` - API implementation
- `src-tauri/src/commands/devtools.rs` - Rust command
- `src-tauri/src/lib.rs` - Command registration
- `src-tauri/capabilities/default.json` - Permissions
