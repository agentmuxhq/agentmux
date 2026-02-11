# AgentMux White/Grey Flashing Screen - Debug Report

**Date:** 2026-02-11
**Version:** 0.22.0
**Issue:** Window flashes white/grey during application startup
**Severity:** High (Poor UX, affects every launch)

---

## Executive Summary

AgentMux exhibits a **white or grey flashing screen** during startup. This is a **timing issue** where the Tauri window becomes visible before the frontend content is loaded and hidden by JavaScript.

**Recent related fixes:**
- ✅ PR #255 (0.21.2) - Fixed dev mode white screen (webview URL issue)
- ✅ PR #254 (0.21.1) - Fixed grey screen (RPC stub missing)
- ❌ **Current flash bug persists** - Different root cause

---

## Root Cause Analysis

### The Problem

The main application window is configured to be **visible immediately** upon creation, but the frontend initialization sequence takes time:

```
Timeline:
0ms   - Tauri creates window → VISIBLE (white/grey background)
10ms  - HTML loads with <body class="init">
50ms  - JavaScript executes
80ms  - initBare() hides body (visibility: hidden)
200ms - Backend ready, fetch client data
500ms - React renders
550ms - initWave() completes
560ms - Finally block shows body (visibility: null)

FLASH OCCURS: 0ms-80ms (window visible, no content)
```

### Contributing Factors

1. **No visibility control on main window**
   - `tauri.conf.json` has no `"visible": false` setting
   - Defaults to visible immediately
   - New windows ARE protected with `.visible(false)` (contrast!)

2. **No CSS protection**
   - `<body class="init">` has **no associated CSS rules**
   - Body is visible with default browser styling until JS hides it

3. **Race condition**
   - Window becomes visible **before** `initBare()` executes
   - JavaScript must download, parse, execute before hiding body
   - Network latency can worsen this (loading external resources)

4. **Plugin interactions**
   - `tauri-plugin-window-state` restores window position/size
   - May trigger additional visibility events

5. **Multi-step async initialization**
   - Must wait for: backend-ready → API setup → data fetch → React render
   - Each step adds delay while window is visible

---

## Current Code Flow

### 1. Window Configuration (`tauri.conf.json`)

```json
{
  "app": {
    "windows": [{
      "title": "AgentMux",
      "width": 1200,
      "height": 800,
      "decorations": true,
      "transparent": false
      // ❌ No "visible": false
    }]
  }
}
```

**Problem:** Window is visible from moment of creation.

### 2. HTML Initial State (`index.html`)

```html
<body class="init" data-colorscheme="dark">
  <div id="main"></div>
  <script type="module" src="/src/tauri-bootstrap.ts"></script>
</body>
```

**Problem:** `.init` class has no CSS rules - does nothing.

### 3. Body Visibility Management (`wave.ts`)

```typescript
// Line 212-214: Hide body during initialization
export function initBare(): Promise<void> {
    document.body.style.visibility = "hidden";
    document.body.style.opacity = "0";
    document.body.classList.add("is-transparent");
    // ... rest of init
}

// Line 287-289: Show body when ready (finally block)
finally {
    document.body.style.visibility = null;
    document.body.style.opacity = null;
    document.body.classList.remove("is-transparent");
}
```

**Problem:** This executes 50-100ms AFTER window is visible.

### 4. New Windows (Working Example)

```typescript
// frontend/app/store/services.ts:193
await getApi().createWindow(newWindowData);
// Tauri creates with .visible(false), shows after init
```

```rust
// src-tauri/src/commands/window.rs:30
let window = tauri::WebviewWindowBuilder::new(
    app,
    label.clone(),
    tauri::WebviewUrl::App(Default::default()),
)
.visible(false) // ✅ Start hidden
// ...
```

**This pattern works!** New windows don't flash. Main window should use same approach.

---

## Existing Logging Framework

### Backend (Rust) - Using `tracing` crate

**Location:** `src-tauri/src/lib.rs:274-303`

```rust
fn init_logging() {
    let log_dir = get_log_dir(); // Platform-specific
    let log_file = log_dir.join("agentmux.log");

    let file_appender = tracing_appender::rolling::never(&log_dir, "agentmux.log");

    let subscriber = tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).ok();
}
```

**Features:**
- ✅ Writes to file (survives app restart)
- ✅ Platform-specific paths:
  - Windows: `C:\Users\<user>\AppData\Local\com.a5af.agentmux\logs\agentmux.log`
  - Linux: `~/.local/share/com.a5af.agentmux/logs/agentmux.log`
  - macOS: `~/Library/Application Support/com.a5af.agentmux/logs/agentmux.log`
- ✅ Non-rotating (simple append)
- ⚠️ Fixed level: `INFO` (no runtime control)

**Current log points:**
```rust
tracing::info!("AgentMux starting"); // lib.rs:300
tracing::info!("dev mode: navigating webview to {}", url); // lib.rs:173
tracing::info!("Rust-native backend initialized successfully"); // lib.rs:186
```

### Frontend (TypeScript) - Custom logging

**Location:** `frontend/tauri-bootstrap.ts:14-26`

```typescript
function debugLog(...args: any[]) {
    console.log(...args);
    if (window.api?.sendLog) {
        const msg = args.map(a => String(a)).join(" ");
        window.api.sendLog(msg).catch(err => {
            console.error("failed to send log to backend", err);
        });
    }
}
```

**Features:**
- ✅ Logs to console (DevTools)
- ✅ Forwards to backend via `sendLog()` RPC
- ✅ Global error handlers (lines 31-36)
- ⚠️ No structured logging
- ⚠️ No log levels

**Current log points:**
```typescript
debugLog("tauri-bootstrap starting"); // tauri-bootstrap.ts:40
debugLog("Init Bare - Tauri mode:", isTauri); // wave.ts:218
debugLog("Init Wave", { clientId, windowId, activeTabId }); // wave.ts:345
debugLog("Wave First Render"); // wave.ts:400
```

---

## Proposed Solutions

### Solution 1: CSS-Based Prevention ⭐ **SIMPLEST**

**Effort:** 5 minutes
**Risk:** Very low
**Effectiveness:** High

Add CSS rule to hide body initially:

```scss
// frontend/app/app.scss
body.init {
    visibility: hidden !important;
    opacity: 0 !important;
    background-color: var(--main-bg-color);
}
```

**How it works:**
1. Body is hidden from HTML load
2. JavaScript removes `.init` class when ready
3. No flash, smooth transition

**Pros:**
- Minimal code change
- No timing dependencies
- Works in all scenarios

**Cons:**
- Relies on CSS loading before window shows
- If CSS fails, window stays hidden

---

### Solution 2: Window Visibility Control ⭐ **MOST RELIABLE**

**Effort:** 30 minutes
**Risk:** Low
**Effectiveness:** Very high

Make main window behave like new windows:

**Step 1:** Update `tauri.conf.json`
```json
{
  "app": {
    "windows": [{
      "visible": false  // ← Add this
    }]
  }
}
```

**Step 2:** Show window when ready (`wave.ts`)
```typescript
// In initWave() after React first render (line 400)
export async function initWave() {
    // ... existing init code ...

    debugLog("Wave First Render");
    const reactElem = createBrowserRouter([ /* ... */ ]);
    root.render(reactElem);

    // Show window after first render
    if (getApi().getIsTauri()) {
        const appWindow = (await import("@tauri-apps/api/window")).getCurrentWindow();
        await appWindow.show();
    }
}
```

**How it works:**
1. Window created but invisible
2. All initialization happens hidden
3. Window shows when content is ready
4. No flash possible

**Pros:**
- Guaranteed no flash
- Same pattern as new windows (proven)
- Clean separation of concerns

**Cons:**
- Must handle show() failure gracefully
- Slightly slower perceived startup

---

### Solution 3: Enhanced Logging Framework ⭐ **FOR DEBUGGING**

**Effort:** 2-3 hours
**Risk:** Low
**Effectiveness:** Enables all future debugging

Implement proper structured logging with runtime control.

#### Backend Enhancement

```rust
// src-tauri/src/backend/logging.rs (new file)
use tracing::{Level, Subscriber};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Layer, Registry};

pub struct LogConfig {
    pub file_level: Level,
    pub console_level: Level,
    pub structured: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file_level: Level::INFO,
            console_level: Level::WARN,
            structured: false,
        }
    }
}

pub fn init_logging_advanced(config: LogConfig) -> Result<(), String> {
    let log_dir = get_log_dir();
    let log_file = log_dir.join("agentmux.log");

    // File appender
    let file_appender = tracing_appender::rolling::daily(&log_dir, "agentmux.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_filter(LevelFilter::from_level(config.file_level));

    // Console layer (optional)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(LevelFilter::from_level(config.console_level));

    // Structured JSON layer (optional)
    let json_layer = if config.structured {
        let json_file = tracing_appender::rolling::daily(&log_dir, "agentmux.json");
        Some(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(json_file)
                .with_filter(LevelFilter::TRACE)
        )
    } else {
        None
    };

    // Combine layers
    let subscriber = Registry::default()
        .with(file_layer)
        .with(console_layer)
        .with(json_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| format!("Failed to set subscriber: {}", e))?;

    Ok(())
}

// Runtime log level control
use std::sync::atomic::{AtomicU8, Ordering};
static LOG_LEVEL: AtomicU8 = AtomicU8::new(Level::INFO as u8);

pub fn set_log_level(level: Level) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn get_log_level() -> Level {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        1 => Level::ERROR,
        2 => Level::WARN,
        3 => Level::INFO,
        4 => Level::DEBUG,
        5 => Level::TRACE,
        _ => Level::INFO,
    }
}

// Conditional logging macro
#[macro_export]
macro_rules! debug_verbose {
    ($($arg:tt)*) => {
        if get_log_level() >= Level::DEBUG {
            tracing::debug!($($arg)*);
        }
    };
}
```

#### Frontend Enhancement

```typescript
// frontend/util/logger.ts (new file)
export enum LogLevel {
    TRACE = 0,
    DEBUG = 1,
    INFO = 2,
    WARN = 3,
    ERROR = 4,
}

class Logger {
    private level: LogLevel = LogLevel.INFO;
    private enabled: boolean = true;

    setLevel(level: LogLevel) {
        this.level = level;
    }

    setEnabled(enabled: boolean) {
        this.enabled = enabled;
    }

    private shouldLog(level: LogLevel): boolean {
        return this.enabled && level >= this.level;
    }

    private async log(level: LogLevel, category: string, ...args: any[]) {
        if (!this.shouldLog(level)) return;

        const timestamp = new Date().toISOString();
        const levelStr = LogLevel[level];
        const msg = `[${timestamp}] [${levelStr}] [${category}] ${args.map(String).join(" ")}`;

        console.log(msg);

        if (window.api?.sendLog) {
            await window.api.sendLog(msg).catch(err => {
                console.error("Failed to send log:", err);
            });
        }
    }

    trace(category: string, ...args: any[]) {
        this.log(LogLevel.TRACE, category, ...args);
    }

    debug(category: string, ...args: any[]) {
        this.log(LogLevel.DEBUG, category, ...args);
    }

    info(category: string, ...args: any[]) {
        this.log(LogLevel.INFO, category, ...args);
    }

    warn(category: string, ...args: any[]) {
        this.log(LogLevel.WARN, category, ...args);
    }

    error(category: string, ...args: any[]) {
        this.log(LogLevel.ERROR, category, ...args);
    }

    // Performance timing
    time(label: string) {
        console.time(label);
    }

    timeEnd(label: string) {
        console.timeEnd(label);
        if (this.shouldLog(LogLevel.DEBUG)) {
            // Log timing to backend
        }
    }
}

export const logger = new Logger();

// Environment-based initialization
export function initLogger() {
    if (import.meta.env.DEV) {
        logger.setLevel(LogLevel.DEBUG);
    } else {
        logger.setLevel(LogLevel.INFO);
    }

    // Allow override via localStorage
    const storedLevel = localStorage.getItem("agentmux-log-level");
    if (storedLevel) {
        logger.setLevel(parseInt(storedLevel) as LogLevel);
    }
}
```

#### Usage Example

```typescript
// wave.ts - Enhanced timing logs
import { logger } from "./util/logger";

export async function initBare(): Promise<void> {
    logger.time("initBare");
    logger.debug("init", "Starting initBare");

    logger.trace("init", "Hiding body");
    document.body.style.visibility = "hidden";
    document.body.style.opacity = "0";

    const isTauri = await getIsTauri();
    logger.debug("init", "isTauri:", isTauri);

    if (isTauri) {
        logger.trace("init", "Waiting for backend-ready event");
        await tauri_event.once("backend-ready");
        logger.debug("init", "Backend ready");
    }

    logger.trace("init", "Loading fonts");
    const fontTimeout = new Promise(resolve => setTimeout(resolve, 2000));
    await Promise.race([document.fonts.ready, fontTimeout]);
    logger.debug("init", "Fonts loaded or timeout");

    logger.timeEnd("initBare");
}
```

**Features:**
- ✅ Runtime enable/disable (zero overhead when off)
- ✅ Multiple log levels (TRACE, DEBUG, INFO, WARN, ERROR)
- ✅ Category-based filtering
- ✅ Performance timing
- ✅ Persistent to backend
- ✅ Environment-aware defaults
- ✅ localStorage override

---

### Solution 4: Timing-Specific Debug Logs

**Effort:** 30 minutes
**Risk:** Very low
**Effectiveness:** Diagnostic only

Add detailed timing logs to understand exact sequence:

```rust
// src-tauri/src/lib.rs
pub fn run() {
    tracing::info!("=== STARTUP SEQUENCE ===");
    tracing::info!("[0ms] Tauri app building");

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            tracing::info!("[??ms] Setup phase - creating window");

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                tracing::info!("[??ms] Dev mode - window created");

                let vite_url = std::env::var("VITE_DEV_URL")
                    .unwrap_or_else(|_| "http://localhost:5173".to_string());
                tracing::info!("[??ms] Navigating to: {}", vite_url);

                window.navigate(vite_url.parse().unwrap()).unwrap();
                tracing::info!("[??ms] Navigation initiated");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::Created => {
                    tracing::info!("[??ms] WindowEvent::Created - {}", window.label());
                }
                WindowEvent::CloseRequested { .. } => {
                    tracing::info!("[??ms] WindowEvent::CloseRequested");
                }
                WindowEvent::Focused(focused) => {
                    tracing::debug!("[??ms] WindowEvent::Focused: {}", focused);
                }
                WindowEvent::Resized(size) => {
                    tracing::trace!("[??ms] WindowEvent::Resized: {:?}", size);
                }
                _ => {}
            }
        });

    tracing::info!("[??ms] Running app");
    builder.run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

```typescript
// frontend/wave.ts
const startTime = performance.now();
function logTiming(label: string) {
    const elapsed = (performance.now() - startTime).toFixed(1);
    console.log(`[${elapsed}ms] ${label}`);
}

export async function initBare(): Promise<void> {
    logTiming("initBare() called");

    logTiming("Hiding body");
    document.body.style.visibility = "hidden";

    logTiming("Checking isTauri");
    const isTauri = await getIsTauri();
    logTiming(`isTauri = ${isTauri}`);

    if (isTauri) {
        logTiming("Waiting for backend-ready");
        await tauri_event.once("backend-ready");
        logTiming("Backend ready received");
    }

    logTiming("Waiting for fonts");
    await Promise.race([document.fonts.ready, fontTimeout]);
    logTiming("Fonts ready");

    logTiming("initBare() complete");
}
```

---

## Would wsh Rewrite in Rust Help?

### Answer: **No - Not for this bug**

**Reasoning:**

1. **Wrong layer:** This bug is in the **Tauri window/frontend initialization**, not in wsh (the CLI sidecar).

2. **wsh role:** wsh is a CLI tool for file operations, remote connections, and RPC communication. It doesn't control window visibility or rendering.

3. **Timing:** The flash occurs during **window creation** (0-100ms), before wsh is even invoked.

4. **Architecture:**
   ```
   AgentMux.exe (Tauri)
   ├── Rust backend (RPC server)
   ├── React frontend (webview)
   └── wsh sidecar (CLI tool) ← Not involved in window init
   ```

### However, wsh Rewrite WOULD Help With:

✅ **Binary size:** 11MB → 2-4MB per platform (-60-80%)
✅ **Distribution size:** 88MB total → 18MB (-80%)
✅ **Build time:** Slightly faster (Rust compiles all at once)
✅ **Code consistency:** All Rust codebase (easier maintenance)
✅ **Performance:** Faster startup, lower memory
✅ **Type safety:** Rust vs Go type system

But for **this specific bug**, the fix is in the Tauri window initialization, not wsh.

---

## Recommended Action Plan

### Phase 1: Immediate Fix (5 minutes)

**Add CSS to hide body initially:**

```scss
// frontend/app/app.scss (add to top)
body.init {
    visibility: hidden !important;
    opacity: 0 !important;
    background-color: var(--main-bg-color);
}
```

```typescript
// frontend/wave.ts (in initWave finally block)
finally {
    document.body.style.visibility = null;
    document.body.style.opacity = null;
    document.body.classList.remove("is-transparent");
    document.body.classList.remove("init"); // ← Add this
}
```

**Test:** Launch app, verify no flash.

---

### Phase 2: Proper Fix (30 minutes)

**Implement window visibility control:**

1. Add `"visible": false` to `tauri.conf.json`
2. Show window after React render in `wave.ts`
3. Add error handling for show() failure
4. Test on all platforms

**Test:** Launch app multiple times, verify smooth appearance.

---

### Phase 3: Enhanced Logging (2-3 hours)

**Implement structured logging framework:**

1. Create `src-tauri/src/backend/logging.rs`
2. Create `frontend/util/logger.ts`
3. Replace all `console.log()` calls
4. Add timing logs to initialization sequence
5. Add RPC command to control log level at runtime

**Test:** Enable DEBUG logs, launch app, analyze timing.

---

### Phase 4: Performance Profiling (1 hour)

**Add detailed timing metrics:**

1. Log every initialization step with timestamps
2. Measure time to first paint
3. Identify slowest operations
4. Optimize critical path

**Test:** Profile with different network conditions, hardware.

---

## Testing Checklist

- [ ] Fresh install (clean state)
- [ ] Cold start (first launch after boot)
- [ ] Warm start (app recently closed)
- [ ] With slow network (throttle in DevTools)
- [ ] With external monitor (different DPI)
- [ ] After window-state restoration (moved/resized)
- [ ] Dev mode vs production build
- [ ] Windows 10, Windows 11
- [ ] Light theme, dark theme

---

## Log Analysis Commands

### Backend Logs

**Windows:**
```powershell
Get-Content "$env:LOCALAPPDATA\com.a5af.agentmux\logs\agentmux.log" -Tail 100 -Wait
```

**Linux:**
```bash
tail -f ~/.local/share/com.a5af.agentmux/logs/agentmux.log
```

**Filter for timing:**
```bash
grep -E "\[.*ms\]" agentmux.log
```

### Frontend Logs

**Chrome DevTools:**
1. Open DevTools (F12)
2. Console tab
3. Filter: `/\[.*ms\]/` (regex)
4. Or filter by source: `wave.ts`, `tauri-bootstrap.ts`

**Performance Timeline:**
1. DevTools → Performance tab
2. Record → Reload page
3. Analyze paint events
4. Check for layout shifts

---

## Success Criteria

**Fix is successful when:**

1. ✅ No visible white/grey flash on startup
2. ✅ Window appears smoothly with content ready
3. ✅ Consistent behavior across cold/warm starts
4. ✅ Works on all supported platforms
5. ✅ No performance regression
6. ✅ Logs show <100ms from window create to content ready

---

## Appendix: File Locations

### Key Files for Debugging

**Window Configuration:**
- `/c/Systems/agentmux/src-tauri/tauri.conf.json` (line 12-32)

**Window Initialization:**
- `/c/Systems/agentmux/src-tauri/src/lib.rs` (line 130-186, 207-256)
- `/c/Systems/agentmux/src-tauri/src/commands/window.rs` (line 30-193)

**Frontend Initialization:**
- `/c/Systems/agentmux/frontend/wave.ts` (line 212-289, 345-410)
- `/c/Systems/agentmux/frontend/tauri-bootstrap.ts` (line 14-68)
- `/c/Systems/agentmux/index.html` (line 11-18)

**Styling:**
- `/c/Systems/agentmux/frontend/app/app.scss` (line 19, 32-34)

**Logging:**
- `/c/Systems/agentmux/src-tauri/src/lib.rs` (line 274-303)

### Log File Locations

**Windows:**
```
C:\Users\<user>\AppData\Local\com.a5af.agentmux\logs\agentmux.log
```

**Linux:**
```
~/.local/share/com.a5af.agentmux/logs/agentmux.log
```

**macOS:**
```
~/Library/Application Support/com.a5af.agentmux/logs/agentmux.log
```

---

## Conclusion

The white/grey flash is a **timing bug** in window visibility management, not a content loading or RPC issue. The fix is straightforward:

1. **Quick fix:** CSS to hide body initially (5 min)
2. **Proper fix:** Window visibility control (30 min)
3. **Debug tool:** Enhanced logging framework (2-3 hours)

**wsh rewrite in Rust won't help with this specific bug**, but would provide other benefits (size, performance, consistency).

Recommend implementing **both Phase 1 (CSS fix) and Phase 3 (logging framework)** in next release to resolve the current issue and enable future debugging.
