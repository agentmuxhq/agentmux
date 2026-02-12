# DevTools Button Investigation

## Problem
DevTools button in WidgetBar doesn't work when clicked.

## Root Cause Analysis

### What I Did Wrong (First Attempt)
In `frontend/util/tauri-api.ts`, I implemented:
```typescript
toggleDevtools: () => {
    const webview = getCurrentWebviewWindow();
    webview.toggleDevtools();  // ❌ THIS METHOD DOESN'T EXIST
}
```

**Issue**: `toggleDevtools()` is NOT a method on Tauri's WebviewWindow API.

### What Actually Exists

**Backend Command** (already implemented):
- File: `src-tauri/src/commands/devtools.rs`
- Command: `toggle_devtools` (Tauri command)
- Registered: `src-tauri/src/lib.rs:72`

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

**Tauri WebviewWindow Methods** (available in Rust):
- `window.open_devtools()` - Opens devtools
- `window.close_devtools()` - Closes devtools
- `window.is_devtools_open()` - Checks if open

**Frontend Access**:
- Must use `invoke("toggle_devtools")` to call the Rust command
- Cannot directly call webview methods from TypeScript

### Correct Implementation

Replace the frontend implementation with:
```typescript
toggleDevtools: () => {
    invoke("toggle_devtools").catch(console.error);
}
```

This calls the registered Tauri command which has access to the window methods.

## Additional Findings

### Menu Integration
The menu system already uses this command:
- File: `src-tauri/src/menu.rs:227-235`
- Menu item: "Toggle DevTools" (Alt+Shift+I / Option+Command+I)
- Calls the same backend method

### Capabilities
Tauri requires permission for devtools:
- Permission: `core:webview:allow-internal-toggle-devtools`
- Status: Included in default permissions (see schema files)
- Feature flag: `devtools` enabled in `Cargo.toml:17`

## Fix Required

File: `frontend/util/tauri-api.ts`
Line: ~245 (in the API object)

Change from:
```typescript
toggleDevtools: () => {
    const webview = getCurrentWebviewWindow();
    webview.toggleDevtools();
},
```

To:
```typescript
toggleDevtools: () => {
    invoke("toggle_devtools").catch(console.error);
},
```

## Why This Works

1. `invoke("toggle_devtools")` calls the Rust function
2. Rust function receives `WebviewWindow` handle automatically (injected by Tauri)
3. Rust calls native `window.open_devtools()` or `window.close_devtools()`
4. Devtools toggle for the current window

## Testing

After fix:
1. Click DevTools button in WidgetBar
2. Should see console log: "Opening devtools for window: main" (or "Closing...")
3. DevTools panel should appear/disappear

## Potential Issues (If Still Doesn't Work)

### 1. Silent Invoke Errors
**Symptom**: Button does nothing, no console errors
**Check**: Look in Rust logs at `~/.waveterm-dev/waveapp.log` or `%APPDATA%/com.a5af.wavemux/logs/`
**Fix**: Add better error handling in frontend

### 2. Window Context Missing
**Symptom**: Error like "window not found"
**Cause**: invoke() might not have window context
**Fix**: Pass window label explicitly: `invoke("toggle_devtools", { window: "main" })`

### 3. Permissions Issue
**Symptom**: Permission denied error
**Check**: Ensure `core:webview:default` in capabilities includes `allow-internal-toggle-devtools`
**Current**: Line 21 in `capabilities/default.json` has `core:webview:default`

### 4. Feature Flag Not Working
**Symptom**: Methods don't exist even though feature is enabled
**Check**: Verify `devtools` feature in Cargo.toml (currently line 17)
**Rebuild**: Try clean rebuild: `cargo clean && npm run build`

### 5. Webview vs Window Confusion
**Symptom**: Command runs but nothing happens
**Cause**: Tauri v2 has both Window and Webview - devtools might be on wrong object
**Check**: The command uses `WebviewWindow` which is correct for Tauri v2

### 6. Release Build Limitation
**Symptom**: Works in dev, not in release
**Check**: The devtools.rs has both debug and release implementations
**Note**: Should work in both (intentional for debugging support)

## Related Files

- `frontend/app/tab/widgetbar.tsx:29` - Button click handler
- `frontend/types/custom.d.ts:101` - API interface definition
- `src-tauri/src/lib.rs:72` - Command registration
- `src-tauri/Cargo.toml:17` - DevTools feature flag
