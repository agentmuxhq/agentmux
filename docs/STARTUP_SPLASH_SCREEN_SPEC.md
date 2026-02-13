# Startup Splash Screen Specification

## Problem Statement

During AgentMux startup, the window flashes multiple different colors before showing the proper UI. This creates a jarring user experience and appears unprofessional.

### Current Behavior (Color Flash Sequence)

1. **Window Opens** → System default background (white on light themes, black on dark themes)
2. **HTML Loads** → Brief flash of browser default background
3. **CSS Loads** → Background changes to `rgb(34, 34, 34)` (--main-bg-color)
4. **React Initializes** → Additional layout shifts as components mount
5. **Backend Connects** → Final UI renders

**Duration:** ~500-2000ms depending on system performance
**Color flashes:** 3-5 different background colors

### Root Cause Analysis

#### 1. Tauri Configuration
```json
// src-tauri/tauri.conf.json
{
  "app": {
    "windows": [{
      "title": "AgentMux",
      "width": 1200,
      "height": 800,
      // ❌ No "visible" property (defaults to true)
      // ❌ No "backgroundColor" property
      // ❌ No splash screen configuration
    }]
  }
}
```

**Issue:** Window shows immediately with system default background, before any HTML/CSS loads.

#### 2. HTML/CSS Initialization
```html
<!-- index.html -->
<body class="init" data-colorscheme="dark">
  <div id="main"></div>
</body>
```

```scss
// frontend/app/app.scss
body {
  background: rgb(from var(--main-bg-color) r g b / var(--window-opacity));
  // ❌ Requires CSS variables to be loaded first
  // ❌ No static fallback color
}
```

**Issue:** Background color depends on CSS variables which load asynchronously.

#### 3. Bootstrap Sequence
```typescript
// frontend/tauri-bootstrap.ts → frontend/tauri-init.ts → frontend/wave.ts
// ❌ No splash screen shown during initialization
// ❌ No "app ready" signal to show window
// ❌ React renders directly into empty #main div
```

**Issue:** No intermediate loading state between window open and React render.

## Proposed Solutions

### Solution 1: Static Inline Splash Screen (Recommended)

**Approach:** Embed a static splash screen directly in `index.html` that matches the landing page logo, then hide it when React is ready.

#### Implementation

**Step 1: Update `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta name="color scheme" content="light dark" />
    <title>AgentMux</title>

    <!-- Inline critical CSS for splash screen -->
    <style>
      /* Reset and base styles */
      html, body {
        margin: 0;
        padding: 0;
        width: 100%;
        height: 100%;
        overflow: hidden;
        background: rgb(34, 34, 34); /* --main-bg-color */
      }

      /* Splash screen container */
      #splash-screen {
        position: fixed;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: rgb(34, 34, 34);
        z-index: 9999;
        transition: opacity 0.3s ease-out;
      }

      #splash-screen.hidden {
        opacity: 0;
        pointer-events: none;
      }

      /* Logo animation */
      #splash-logo {
        width: 120px;
        height: 120px;
        animation: float 3s ease-in-out infinite;
      }

      @keyframes float {
        0%, 100% { transform: translateY(0); }
        50% { transform: translateY(-8px); }
      }

      /* Loading indicator */
      #splash-loading {
        margin-top: 32px;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        font-size: 14px;
        color: rgba(255, 255, 255, 0.6);
        letter-spacing: 0.5px;
      }

      /* Spinner */
      .spinner {
        width: 20px;
        height: 20px;
        margin: 24px auto 0;
        border: 2px solid rgba(88, 193, 66, 0.2);
        border-top-color: rgb(88, 193, 66);
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
      }

      @keyframes spin {
        to { transform: rotate(360deg); }
      }

      /* Hide main during load */
      #main {
        opacity: 0;
        transition: opacity 0.3s ease-in;
      }

      #main.ready {
        opacity: 1;
      }
    </style>

    <!-- External stylesheets load after inline CSS -->
    <link rel="stylesheet" href="/fontawesome/css/fontawesome.min.css" />
    <link rel="stylesheet" href="/fontawesome/css/brands.min.css" />
    <link rel="stylesheet" href="/fontawesome/css/solid.min.css" />
    <link rel="stylesheet" href="/fontawesome/css/sharp-solid.min.css" />
    <link rel="stylesheet" href="/fontawesome/css/sharp-regular.min.css" />
    <link rel="stylesheet" href="/fontawesome/css/custom-icons.min.css" />
    <script type="module" src="frontend/tauri-bootstrap.ts"></script>
  </head>
  <body class="init" data-colorscheme="dark">
    <!-- Splash Screen -->
    <div id="splash-screen">
      <svg id="splash-logo" viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="splash-gradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" style="stop-color:#1F4D22" />
            <stop offset="100%" style="stop-color:#58C142" />
          </linearGradient>
        </defs>
        <rect width="200" height="200" rx="40" fill="#000"/>
        <path d="M 30 95 C 50 55, 85 55, 100 82 C 115 109, 150 109, 170 69 L 170 82 C 150 122, 115 122, 100 95 C 85 68, 50 68, 30 108 Z" fill="url(#splash-gradient)" opacity="0.85"/>
        <path d="M 30 118 C 50 78, 85 78, 100 105 C 115 132, 150 132, 170 92 L 170 105 C 150 145, 115 145, 100 118 C 85 91, 50 91, 30 131 Z" fill="url(#splash-gradient)" opacity="0.45"/>
        <path d="M 100 78 L 112 95 L 100 112 L 88 95 Z" fill="#58C142" opacity="0.8"/>
        <path d="M 100 85 L 107 95 L 100 105 L 93 95 Z" fill="#ffffff" opacity="0.6"/>
      </svg>
      <div id="splash-loading">Starting AgentMux...</div>
      <div class="spinner"></div>
    </div>

    <!-- Main app container -->
    <div id="main"></div>
  </body>
</html>
```

**Step 2: Update `frontend/wave.ts`**

```typescript
// After React app mounts successfully
async function main(): Promise<void> {
    // ... existing initialization code ...

    const mainDiv = document.getElementById("main");
    if (mainDiv == null) {
        throw new Error("Cannot find #main div");
    }

    // Mount React app
    const app = createElement(App);
    const root = createRoot(mainDiv);
    root.render(app);

    // Hide splash screen after React renders
    requestAnimationFrame(() => {
        const splashScreen = document.getElementById("splash-screen");
        const mainDiv = document.getElementById("main");

        if (splashScreen) {
            splashScreen.classList.add("hidden");
            setTimeout(() => {
                splashScreen.remove(); // Clean up after fade
            }, 300);
        }

        if (mainDiv) {
            mainDiv.classList.add("ready");
        }
    });
}
```

**Step 3: Update `src-tauri/tauri.conf.json`**

```json
{
  "app": {
    "windows": [{
      "title": "AgentMux",
      "width": 1200,
      "height": 800,
      "minWidth": 400,
      "minHeight": 300,
      "decorations": false,
      "transparent": false,
      "backgroundColor": "#222222",  // Matches --main-bg-color
      "dragDropEnabled": false
    }]
  }
}
```

#### Advantages
- ✅ **Zero flashing**: Background stays consistent from window open to app ready
- ✅ **No dependencies**: Works even if JavaScript fails to load
- ✅ **Minimal code**: ~50 lines of inline CSS, no new files
- ✅ **Matches brand**: Uses official AgentMux logo from landing page
- ✅ **Progressive enhancement**: Gracefully degrades if styles fail

#### Disadvantages
- ⚠️ Increases `index.html` size by ~2KB (negligible)
- ⚠️ Logo SVG duplicated (once in HTML, once in landing page)

---

### Solution 2: Tauri Splash Screen API (Alternative)

**Approach:** Use Tauri's built-in splash screen window.

#### Implementation

**Step 1: Create splash screen HTML**

```html
<!-- src-tauri/splash.html -->
<!DOCTYPE html>
<html>
<head>
  <style>
    body {
      margin: 0;
      padding: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
      background: rgb(34, 34, 34);
      font-family: -apple-system, sans-serif;
    }
    /* ... logo and spinner styles ... */
  </style>
</head>
<body>
  <!-- Logo SVG here -->
</body>
</html>
```

**Step 2: Update `src-tauri/tauri.conf.json`**

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "visible": false,  // Don't show until ready
        "title": "AgentMux",
        "width": 1200,
        "height": 800
      },
      {
        "label": "splashscreen",
        "visible": true,
        "url": "splash.html",
        "decorations": false,
        "transparent": false,
        "alwaysOnTop": true,
        "center": true,
        "width": 400,
        "height": 400
      }
    ]
  }
}
```

**Step 3: Update `src-tauri/src/lib.rs`**

```rust
use tauri::{AppHandle, Manager};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle();

            // Wait for frontend ready signal
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // Show main window
                if let Some(main) = app_handle.get_webview_window("main") {
                    main.show().unwrap();
                }

                // Close splash screen
                if let Some(splash) = app_handle.get_webview_window("splashscreen") {
                    splash.close().unwrap();
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### Advantages
- ✅ **Native Tauri pattern**: Uses built-in window management
- ✅ **Separate concerns**: Splash screen is its own window
- ✅ **Precise timing**: Show main window exactly when ready

#### Disadvantages
- ⚠️ **More complex**: Requires window management logic in Rust
- ⚠️ **Additional window**: Brief flash as windows switch
- ⚠️ **Timing issues**: Hard to detect "truly ready" state from backend
- ⚠️ **New files**: Requires `splash.html` maintenance

---

### Solution 3: Hybrid (Best of Both)

**Approach:** Combine static inline splash with hidden window.

#### Implementation

Use Solution 1 (inline splash) + set `visible: false` in tauri.conf.json:

```json
{
  "app": {
    "windows": [{
      "visible": false,  // Hide until React ready
      "backgroundColor": "#222222"
    }]
  }
}
```

```rust
// src-tauri/src/commands/platform.rs
#[tauri::command]
pub async fn app_ready(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

```typescript
// frontend/wave.ts
async function main(): Promise<void> {
    // ... initialization ...
    root.render(app);

    // Signal Tauri to show window
    await invoke("app_ready");

    // Then hide inline splash
    document.getElementById("splash-screen")?.classList.add("hidden");
}
```

#### Advantages
- ✅ **Best UX**: Zero flashing, smooth transition
- ✅ **Fail-safe**: Inline splash works even if Tauri signal fails
- ✅ **Clean code**: Simple Rust command, minimal frontend changes

#### Disadvantages
- ⚠️ **Slightly more complex**: Requires Rust command + frontend call
- ⚠️ **Window management**: Must handle show window logic

---

## Recommendation

**Implement Solution 1: Static Inline Splash Screen**

### Rationale

1. **Simplest implementation**: No new files, minimal code changes
2. **Immediate benefit**: Eliminates color flashing with ~50 lines of CSS
3. **Zero dependencies**: Works even if JavaScript/React fails
4. **Maintainable**: All splash code in one place (`index.html`)
5. **Progressive enhancement**: Can upgrade to Solution 3 later if needed

### Implementation Checklist

- [ ] Add inline CSS and splash HTML to `index.html`
- [ ] Update `src-tauri/tauri.conf.json` backgroundColor
- [ ] Add splash hide logic to `frontend/wave.ts`
- [ ] Test on Windows (cold start, fast machine, slow machine)
- [ ] Test error scenarios (backend fails to start)
- [ ] Verify splash removes cleanly after transition

---

## Edge Cases

### 1. Backend Fails to Start

**Current behavior:** White screen, no feedback

**With splash:** Splash stays visible, shows "Starting AgentMux..." forever

**Solution:** Add timeout to show error message:

```typescript
// frontend/tauri-bootstrap.ts
const BACKEND_TIMEOUT = 30000; // 30 seconds

setTimeout(() => {
    if (!backendReady) {
        document.getElementById("splash-loading").textContent =
            "Backend failed to start. Check logs.";
        document.querySelector(".spinner")?.remove();
    }
}, BACKEND_TIMEOUT);
```

### 2. React Fails to Mount

**Current behavior:** Empty window

**With splash:** Splash stays visible

**Solution:** Use window.onerror to detect failures:

```html
<script>
window.onerror = function(msg, url, line) {
    document.getElementById("splash-loading").textContent =
        "App failed to load. Error: " + msg;
    document.querySelector(".spinner")?.remove();
};
</script>
```

### 3. Slow Network (Dev Mode)

**Current behavior:** Long delay before UI shows

**With splash:** Spinner indicates loading

**Solution:** Already handled by Solution 1

### 4. Theme Switching

**Current behavior:** Flash when switching dark/light theme

**With splash:** Not affected (splash is dark by default)

**Solution:** Consider detecting system theme preference:

```html
<script>
// Detect system theme
const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
if (!isDark) {
    document.getElementById("splash-screen").style.background = "#f0f0f0";
    // Update logo colors for light theme
}
</script>
```

---

## Testing Plan

### Manual Testing

1. **Cold start** (first launch after install)
2. **Warm start** (launch with cache)
3. **Slow machine** (simulate with CPU throttling)
4. **Network delay** (disable cache, throttle network in DevTools)
5. **Error scenarios** (kill backend process before start)

### Automated Testing

```typescript
// e2e/splash.spec.ts
import { test, expect } from '@playwright/test';

test('splash screen appears on startup', async ({ page }) => {
    await page.goto('/');
    const splash = page.locator('#splash-screen');
    await expect(splash).toBeVisible();
});

test('splash screen hides when app ready', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('#main.ready', { timeout: 10000 });
    const splash = page.locator('#splash-screen');
    await expect(splash).toHaveClass(/hidden/);
});

test('splash screen shows error on backend failure', async ({ page }) => {
    // Mock backend failure
    await page.route('ws://localhost:*', route => route.abort());
    await page.goto('/');
    await page.waitForTimeout(30000);
    const loadingText = page.locator('#splash-loading');
    await expect(loadingText).toContainText('failed');
});
```

---

## Performance Impact

### Before (Current)

- **Time to visible UI:** 500-2000ms
- **Color flashes:** 3-5 flashes
- **User perception:** Poor, unprofessional

### After (Solution 1)

- **Time to visible splash:** <50ms (inline CSS)
- **Time to visible UI:** Same (500-2000ms)
- **Color flashes:** 0 ✅
- **User perception:** Professional, polished

### Overhead

- **HTML size:** +2KB (splash screen HTML/CSS)
- **Parse time:** +5ms (inline CSS parsing)
- **Runtime overhead:** 0 (removed after fade)

---

## Future Enhancements

1. **Animated logo**: Add subtle pulse/glow effect
2. **Progress indicator**: Show actual loading stages
3. **Theme detection**: Auto-switch splash theme based on system preference
4. **Skip splash**: Fast-path for warm starts (check localStorage flag)
5. **Custom splash**: Allow users to disable or customize splash screen

---

## References

- [Tauri Window Configuration](https://tauri.app/v1/api/config/#windowconfig)
- [Tauri Splash Screen Guide](https://tauri.app/v1/guides/features/splashscreen)
- [Web Performance: Critical Rendering Path](https://web.dev/critical-rendering-path/)
- [AgentMux Landing Page](https://github.com/a5af/wavemux/blob/main/landing/index.html) (logo source)

---

**Document Version:** 1.0.0
**Author:** AgentX
**Date:** 2026-02-13
**Status:** Proposed
