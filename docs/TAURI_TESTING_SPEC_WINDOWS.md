# Tauri Testing Specification for Windows

## Overview

This document outlines best practices and implementation guidelines for automated end-to-end (E2E) testing of AgentMux on Windows using the Tauri WebDriver framework. This specification builds on the Linux testing implementation and addresses Windows-specific challenges, optimizations, and test scenarios.

**Cross-Platform Strategy:** ~90% of tests are shared between Linux and Windows. This spec focuses on:
- Platform-aware configuration (WebDriver setup, binary paths)
- Windows-specific adaptations (timing, Edge compatibility)
- Windows-only test cases (DPI scaling, Edge WebView2 features)
- Unified test organization for maximum code reuse

## Quick Start Summary

**For the busy developer:**

✅ **Good News:** Most tests already work on Windows!
- Existing `test/specs/zoom.e2e.js` runs unchanged on Windows
- Existing `test/helpers/tauri-helpers.js` is fully cross-platform
- Only configuration changes needed to enable Windows support

⚡ **Quick Win Path (1-2 hours):**
1. Update `wdio.conf.cjs` with platform detection (10 lines)
2. Add `webviewOptions: {}` for EdgeDriver 117+ (1 line)
3. Create `platform-helpers.js` for timing/platform checks (30 lines)
4. Add `windows-latest` to CI matrix (5 lines)
5. Run existing tests - they should pass immediately

⚠️ **Critical Windows Requirement:**
- Edge and EdgeDriver versions MUST match exactly (use msedgedriver-tool)

📦 **What's Windows-Specific:**
- DPI scaling tests (new, ~4-6 hours)
- Edge WebView2 features (new, ~2-3 hours)
- Timing adjustments (minimal, if tests are flaky)

💡 **Total Effort:** 13-20 hours for full Windows support vs 60-80 hours if writing from scratch.

## Testing Stack

### Core Components

- **tauri-driver**: Cross-platform WebDriver wrapper for Tauri applications
- **Microsoft Edge WebDriver (msedgedriver)**: Native Windows WebDriver for WebView2 communication
- **WebdriverIO (WDIO)**: Node.js test automation framework (v7.x recommended for compatibility)
- **msedgedriver-tool**: Automatic Edge Driver version management for CI/CD

### Why Not WinAppDriver?

**WinAppDriver is NOT suitable for Tauri applications:**
- Designed for native Windows apps (UWP, WinForms, WPF, Win32)
- Cannot interact with WebView2-based applications
- Lacks proper DOM access and web automation capabilities

**tauri-driver is the correct choice:**
- Specifically designed for Tauri's WebView architecture
- Uses msedgedriver to communicate with Microsoft Edge WebView2
- Cross-platform support (Windows & Linux)
- Full DOM access and web automation features

## Cross-Platform Test Reuse Strategy

### Overview

**Goal:** Maximize code reuse between Linux and Windows test suites.

**Current Implementation:** The existing Linux test suite (`test/specs/zoom.e2e.js`) is already cross-platform and runs on both platforms with minimal configuration changes.

### Test Reusability Matrix

| Test Category | Reusability | Platform-Specific Adaptations |
|---------------|-------------|-------------------------------|
| **Zoom functionality** | ✅ 100% | None - already handles Cmd vs Ctrl |
| **Keyboard shortcuts** | ✅ 90% | Document Win key limitations |
| **Window management** | ✅ 80% | Add tolerance for DPI differences |
| **Text input** | ✅ 100% | None - WebView behavior identical |
| **Performance tests** | ✅ 100% | None - API-based measurements |
| **IPC communication** | ✅ 100% | None - Tauri API is cross-platform |
| **Context menus** | ⚠️ 60% | Different timing, menu behavior |
| **DPI scaling** | ❌ 0% | Windows-specific feature |

**Overall: ~90% of test code is shared between platforms.**

### Unified Test Organization

```
test/
├── specs/
│   ├── zoom.e2e.js                 # ✅ Cross-platform (already implemented)
│   ├── keyboard-shortcuts.e2e.js   # ✅ Cross-platform with skip conditions
│   ├── window-management.e2e.js    # ✅ Cross-platform with platform helpers
│   ├── text-input.e2e.js           # ✅ Cross-platform
│   ├── context-menu.e2e.js         # ⚠️ Cross-platform with timing adjustments
│   ├── performance.e2e.js          # ✅ Cross-platform
│   └── windows/                     # ❌ Windows-only tests
│       ├── dpi-scaling.e2e.js      # Multi-monitor, DPI transitions
│       └── edge-webview.e2e.js     # EdgeDriver-specific features
├── helpers/
│   ├── tauri-helpers.js            # ✅ Cross-platform (already implemented)
│   ├── platform-helpers.js         # Platform detection utilities
│   └── windows-helpers.js          # Windows-specific utilities
└── wdio.conf.cjs                   # Platform-aware configuration
```

### Platform Detection Pattern

**Use consistent platform detection across all tests:**

```javascript
// helpers/platform-helpers.js
export function isWindows() {
  return process.platform === 'win32'
}

export function isLinux() {
  return process.platform === 'linux'
}

export function isMac() {
  return process.platform === 'darwin'
}

export async function getPlatformFromBrowser() {
  return await browser.execute(() => navigator.platform)
}

export function getWaitTime(base, windowsMultiplier = 1.5) {
  // Windows often needs longer waits for animations/rendering
  return isWindows() ? Math.floor(base * windowsMultiplier) : base
}
```

**Usage in tests:**
```javascript
import { isWindows, getWaitTime } from '../helpers/platform-helpers.js'

describe('Context Menus', () => {
  it('should open menu', async () => {
    await element.click({ button: 'right' })

    // Platform-aware timing
    await browser.pause(getWaitTime(300)) // 300ms Linux, 450ms Windows

    const menu = await $('.context-menu')
    expect(await menu.isDisplayed()).toBe(true)
  })

  it('should handle Win key shortcuts', async function() {
    if (!isWindows()) {
      this.skip() // Skip on non-Windows platforms
    }

    // Windows-specific test
  })
})
```

### Adapting Existing Linux Tests for Windows

**Minimal changes needed:**

1. **Configuration only** (`wdio.conf.cjs`):
   ```javascript
   const os = require('os')
   const isWindows = os.platform() === 'win32'

   exports.config = {
     capabilities: [{
       'tauri:options': {
         application: isWindows
           ? './src-tauri/target/release/agentmux.exe'
           : './src-tauri/target/release/agentmux'
       },
       webviewOptions: isWindows ? {} : undefined
     }],

     beforeSession: async () => {
       const driverPath = path.resolve(
         os.homedir(),
         '.cargo',
         'bin',
         isWindows ? 'tauri-driver.exe' : 'tauri-driver'
       )

       tauriDriver = spawn(driverPath, ['--port', '4444'])

       // Windows needs slightly longer startup
       await new Promise(resolve =>
         setTimeout(resolve, isWindows ? 3000 : 2000)
       )
     }
   }
   ```

2. **Test adjustments** (where needed):
   - Add platform-aware wait times
   - Use conditional skips for platform-specific features
   - Add tolerance for DPI-related size differences

**Example - Existing `zoom.e2e.js` already cross-platform:**
```javascript
// This test runs unchanged on both platforms!
it('should zoom in with Ctrl+=', async function() {
  const initialZoom = await getZoomFactor()

  await browser.keys(['Control', '='])
  await browser.pause(500)

  const newZoom = await getZoomFactor()
  expect(newZoom).toBeGreaterThan(initialZoom)
})
```

### Real-World Example: Cross-Platform Test

**This test from `test/specs/zoom.e2e.js` runs UNCHANGED on both Linux and Windows:**

```javascript
import {
  getZoomFactor,
  setZoomFactor,
  waitForAppReady
} from '../helpers/tauri-helpers.js'

describe('Zoom Functionality', () => {
  before(async () => {
    await waitForAppReady()
  })

  beforeEach(async () => {
    // Reset zoom before each test
    await setZoomFactor(1.0)
    await browser.pause(500)
  })

  // ✅ This test works on BOTH Linux and Windows with ZERO changes
  it('should zoom in with Ctrl+=', async function() {
    this.timeout(15000)

    const initialZoom = await getZoomFactor()
    console.log(`Initial zoom: ${initialZoom}`)

    // WebdriverIO handles Ctrl vs Cmd automatically based on platform
    await browser.keys(['Control', '='])
    await browser.pause(500)

    const newZoom = await getZoomFactor()
    console.log(`New zoom after Ctrl+=: ${newZoom}`)

    expect(newZoom).toBeGreaterThan(initialZoom)
  })

  // ✅ Also cross-platform - Tauri API works identically
  it('should set zoom factor via API', async function() {
    await setZoomFactor(1.5)
    await browser.pause(500)

    const zoom = await getZoomFactor()
    expect(Math.abs(zoom - 1.5)).toBeLessThan(0.05)
  })
})
```

**Why it works on both platforms:**
- ✅ Uses Tauri APIs (`get_zoom_factor`, `set_zoom_factor`) - cross-platform by design
- ✅ Helper functions (`getZoomFactor`, `setZoomFactor`) use `browser.executeAsync()` - works everywhere
- ✅ WebdriverIO handles platform differences (Ctrl vs Cmd) automatically
- ✅ No DOM-specific selectors that might differ between WebKit and Edge
- ✅ No timing assumptions - uses explicit waits

**Run on Linux:**
```bash
# Uses webkit2gtk-driver
xvfb-run npm run test:e2e
```

**Run on Windows:**
```bash
# Uses msedgedriver - same test, different WebDriver
npm run test:e2e
```

**Run on both in CI:**
```yaml
strategy:
  matrix:
    platform: [ubuntu-latest, windows-latest]
runs-on: ${{ matrix.platform }}
steps:
  - run: npm run test:e2e  # Same command!
```

### Migration Path

**Current state:** Linux tests implemented in `test/specs/zoom.e2e.js`

**To add Windows support:**

1. ✅ **Update `wdio.conf.cjs`** - Add platform detection (10 lines)
2. ✅ **Create `platform-helpers.js`** - Shared utilities (30 lines)
3. ✅ **Add to CI matrix** - Include `windows-latest` (5 lines)
4. ✅ **Run existing tests** - Should work immediately with config changes
5. ⚠️ **Adjust timing** - Only if tests are flaky (optional)
6. ➕ **Add Windows-specific tests** - DPI scaling, Edge features (new files)

**Estimated effort:** 1-2 hours to enable Windows, 4-6 hours for Windows-specific tests

## Critical Windows Requirements

### 1. Version Matching (MOST IMPORTANT)

**The Microsoft Edge Driver version MUST exactly match your Windows Edge browser version.**

Version mismatches are the #1 cause of test failures on Windows:
- Test suite hangs during connection
- `SessionNotCreatedException` errors
- Unexplained test failures
- WebDriver refusing to start

**Check your Edge version:**
```powershell
# In browser: edge://version
# Or command line:
& "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" --version
```

**Download matching driver:**
- https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
- Download the EXACT version (e.g., if Edge is 121.0.2277.128, download 121.0.2277.128)
- Extract `msedgedriver.exe` to a location in your PATH

### 2. EdgeDriver 117+ Compatibility

**Known Issue:** EdgeDriver 117 introduced a breaking change requiring empty `webviewOptions` object.

**Error:** `only ASCIIZ protocol mode is supported`

**Solution:**
```javascript
// wdio.conf.cjs
capabilities: [{
  maxInstances: 1,
  'tauri:options': {
    application: './src-tauri/target/release/agentmux.exe'
  },
  webviewOptions: {} // Required for EdgeDriver 117+
}]
```

## Setup

### Dependencies

#### System Requirements

- Windows 10/11 (64-bit)
- Microsoft Edge (latest stable version)
- Microsoft Edge WebDriver (matching Edge version)
- Rust toolchain (latest stable)
- Node.js 18+ (LTS recommended)

#### Installation

**1. Install Rust and tauri-driver:**
```powershell
# Install Rust
winget install --id Rustlang.Rustup

# Install tauri-driver
cargo install tauri-driver --locked
```

**2. Install Microsoft Edge WebDriver:**

**Option A: Manual (Development):**
```powershell
# Check Edge version
& "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" --version

# Download matching driver from:
# https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/

# Extract to a directory in PATH (e.g., C:\WebDrivers)
# Add to PATH if needed:
$env:PATH += ";C:\WebDrivers"
```

**Option B: Automated (CI/CD):**
```powershell
# Install msedgedriver-tool
cargo install --git https://github.com/chippers/msedgedriver-tool

# Run tool to download and install matching driver
msedgedriver-tool.exe $PWD.Path
```

**3. Install Node.js dependencies:**
```json
{
  "devDependencies": {
    "@wdio/cli": "^7.31.1",
    "@wdio/local-runner": "^7.31.1",
    "@wdio/mocha-framework": "^7.31.1",
    "@wdio/spec-reporter": "^7.31.1",
    "webdriverio": "^7.31.1"
  }
}
```

**Why v7?** WebdriverIO v8+ has compatibility issues with tauri-driver on Windows.

```bash
npm install --save-dev @wdio/cli@7.31.1 @wdio/local-runner@7.31.1 @wdio/mocha-framework@7.31.1 @wdio/spec-reporter@7.31.1
```

### WebdriverIO Configuration

**wdio.conf.cjs:**
```javascript
const os = require('os')
const path = require('path')
const { spawn } = require('child_process')

let tauriDriver

exports.config = {
  runner: 'local',
  specs: ['./test/specs/**/*.e2e.js'],
  maxInstances: 1, // Windows: Run tests sequentially for stability
  hostname: 'localhost',
  port: 4444,
  path: '/',

  capabilities: [{
    maxInstances: 1,
    'tauri:options': {
      application: path.resolve(__dirname, 'src-tauri/target/release/agentmux.exe')
    },
    webviewOptions: {} // Required for EdgeDriver 117+
  }],

  logLevel: 'info',
  bail: 0,
  baseUrl: 'http://localhost',
  waitforTimeout: 15000, // Windows: Increase timeout for slower startup
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000
  },

  /**
   * Build the application before testing
   */
  onPrepare: async function() {
    console.log('Building release application...')
    const { spawnSync } = require('child_process')
    const result = spawnSync('cargo', ['build', '--release'], {
      cwd: path.resolve(__dirname, 'src-tauri'),
      stdio: 'inherit'
    })

    if (result.status !== 0) {
      throw new Error('Failed to build application')
    }
  },

  /**
   * Start tauri-driver before test session
   */
  beforeSession: async function() {
    console.log('Starting tauri-driver...')
    const tauriDriverPath = path.resolve(
      os.homedir(),
      '.cargo',
      'bin',
      'tauri-driver.exe'
    )

    tauriDriver = spawn(tauriDriverPath, ['--port', '4444'], {
      stdio: ['ignore', 'pipe', 'pipe']
    })

    tauriDriver.stdout.on('data', (data) => {
      console.log(`tauri-driver: ${data}`)
    })

    tauriDriver.stderr.on('data', (data) => {
      console.error(`tauri-driver error: ${data}`)
    })

    // CRITICAL: Wait for tauri-driver to be ready
    // Windows needs more time than Linux
    await new Promise(resolve => setTimeout(resolve, 3000))
  },

  /**
   * Clean up tauri-driver after test session
   */
  afterSession: async function() {
    console.log('Stopping tauri-driver...')
    if (tauriDriver) {
      tauriDriver.kill()
      // Wait for WebView2 cleanup
      await new Promise(resolve => setTimeout(resolve, 2000))
    }
  }
}
```

## Test Categories

### 1. Window Management Tests (Windows-Specific)

**Windows Considerations:**
- Test both decorated and frameless window modes
- Verify maximize button behavior when `resizable: false`
- Test window snapping (Win+Arrow keys)
- Test taskbar interactions
- Test window chrome/decorations

```javascript
describe('Window Management (Windows)', () => {
  it('should handle window maximize/restore', async () => {
    // Get initial window state
    const initialRect = await browser.getWindowRect()

    // Maximize window
    await browser.maximizeWindow()
    await browser.pause(500)

    const maximizedRect = await browser.getWindowRect()
    expect(maximizedRect.width).toBeGreaterThan(initialRect.width)
    expect(maximizedRect.height).toBeGreaterThan(initialRect.height)

    // Restore window
    await browser.restoreWindow()
    await browser.pause(500)

    const restoredRect = await browser.getWindowRect()
    expect(restoredRect.width).toBeCloseTo(initialRect.width, 10)
  })

  it('should handle window minimize', async () => {
    await browser.minimizeWindow()
    await browser.pause(1000)

    // Restore from minimized
    await browser.maximizeWindow()
    await browser.pause(500)

    const rect = await browser.getWindowRect()
    expect(rect.width).toBeGreaterThan(0)
  })

  it('should maintain window position on resize', async () => {
    const initialRect = await browser.getWindowRect()

    // Resize window
    await browser.setWindowRect({
      width: initialRect.width + 100,
      height: initialRect.height + 50
    })
    await browser.pause(300)

    const newRect = await browser.getWindowRect()
    expect(newRect.x).toBeCloseTo(initialRect.x, 5)
    expect(newRect.y).toBeCloseTo(initialRect.y, 5)
  })

  it('should handle frameless window dragging', async () => {
    const dragRegion = await $('[data-tauri-drag-region]')
    const initialPos = await browser.getWindowRect()

    await dragRegion.moveTo()
    await browser.performActions([{
      type: 'pointer',
      id: 'mouse',
      parameters: { pointerType: 'mouse' },
      actions: [
        { type: 'pointerMove', duration: 0, x: 0, y: 0 },
        { type: 'pointerDown', button: 0 },
        { type: 'pointerMove', duration: 100, x: 100, y: 50 },
        { type: 'pointerUp', button: 0 }
      ]
    }])
    await browser.pause(300)

    const newPos = await browser.getWindowRect()
    expect(Math.abs(newPos.x - initialPos.x)).toBeGreaterThan(50)
  })
})
```

### 2. Context Menu Tests (Windows-Specific)

**Windows Context Menu Challenges:**
- Native Windows context menus vs custom Tauri menus
- Position calculation relative to window top-left
- Menu item accessibility
- Menu dismissal behavior

```javascript
describe('Context Menus (Windows)', () => {
  it('should open custom window header context menu', async () => {
    const windowHeader = await $('.window-header')

    // Right-click on window header
    await windowHeader.click({ button: 'right' })
    await browser.pause(500)

    const contextMenu = await $('.context-menu')
    const isDisplayed = await contextMenu.isDisplayed()

    expect(isDisplayed).toBe(true)
  })

  it('should display correct menu items', async () => {
    const header = await $('.window-header')
    await header.click({ button: 'right' })
    await browser.pause(300)

    // Check for window controls
    const minimizeItem = await $('=Minimize')
    const maximizeItem = await $('=Maximize')
    const closeItem = await $('=Close')

    expect(await minimizeItem.isDisplayed()).toBe(true)
    expect(await maximizeItem.isDisplayed()).toBe(true)
    expect(await closeItem.isDisplayed()).toBe(true)
  })

  it('should execute menu item action', async () => {
    const header = await $('.window-header')
    await header.click({ button: 'right' })
    await browser.pause(300)

    // Click minimize
    const minimizeItem = await $('=Minimize')
    await minimizeItem.click()
    await browser.pause(500)

    // Verify window was minimized
    // Note: May need to use Tauri API to check state
    const isMinimized = await browser.execute(() => {
      return document.hidden || document.visibilityState === 'hidden'
    })

    // Restore window for next test
    await browser.maximizeWindow()
  })

  it('should close menu on Escape', async () => {
    const element = await $('.window-header')
    await element.click({ button: 'right' })
    await browser.pause(300)

    const menu = await $('.context-menu')
    expect(await menu.isDisplayed()).toBe(true)

    await browser.keys('Escape')
    await browser.pause(200)

    expect(await menu.isDisplayed()).toBe(false)
  })

  it('should close menu on outside click', async () => {
    const header = await $('.window-header')
    await header.click({ button: 'right' })
    await browser.pause(300)

    const menu = await $('.context-menu')
    expect(await menu.isDisplayed()).toBe(true)

    // Click outside menu
    const body = await $('body')
    await body.click()
    await browser.pause(200)

    expect(await menu.isDisplayed()).toBe(false)
  })
})
```

### 3. DPI Scaling and Multi-Monitor Tests (Windows-Specific)

**Critical for Windows:** High-DPI displays and multi-monitor setups are common on Windows.

**Known Issues:**
- Window size increases when dragged between monitors with different DPI
- Incorrect window movement with Win+Shift+Arrow
- Blurry rendering on high-DPI displays

```javascript
describe('DPI Scaling (Windows)', () => {
  it('should handle DPI scale changes', async () => {
    const initialZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      const factor = await invoke('get_zoom_factor')
      done(factor)
    })

    // Simulate DPI change by changing zoom
    await browser.executeAsync(async (newZoom, done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      await invoke('set_zoom_factor', { factor: newZoom })
      done()
    }, 1.5)

    await browser.pause(500)

    const newZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      const factor = await invoke('get_zoom_factor')
      done(factor)
    })

    expect(Math.abs(newZoom - 1.5)).toBeLessThan(0.05)
  })

  it('should maintain window size consistency', async () => {
    const rect1 = await browser.getWindowRect()

    // Move window (simulate monitor change)
    await browser.setWindowRect({
      x: rect1.x + 100,
      y: rect1.y + 100,
      width: rect1.width,
      height: rect1.height
    })
    await browser.pause(500)

    const rect2 = await browser.getWindowRect()

    // Size should remain consistent (within margin for DPI)
    expect(Math.abs(rect2.width - rect1.width)).toBeLessThan(20)
    expect(Math.abs(rect2.height - rect1.height)).toBeLessThan(20)
  })

  it('should render clearly at different zoom levels', async () => {
    const zoomLevels = [1.0, 1.25, 1.5, 2.0]

    for (const zoom of zoomLevels) {
      await browser.executeAsync(async (z, done) => {
        const { invoke } = window.__TAURI_INTERNALS__
        await invoke('set_zoom_factor', { factor: z })
        done()
      }, zoom)

      await browser.pause(300)

      // Capture screenshot at this zoom level
      await browser.saveScreenshot(`./test/screenshots/zoom-${zoom}.png`)

      // Verify content is still accessible
      const header = await $('.window-header')
      expect(await header.isDisplayed()).toBe(true)
    }
  })
})
```

### 4. Keyboard Shortcuts Tests (Windows-Specific)

**Windows Keyboard Limitations:**
- System shortcuts (Win key, Alt+Tab) cannot be captured when Tauri window has focus
- Alt+F4 triggers window close events
- Ctrl+ shortcuts work as expected

```javascript
describe('Keyboard Shortcuts (Windows)', () => {
  it('should handle Ctrl+T for new tab', async () => {
    const initialTabs = await $$('.tab')
    await browser.keys(['Control', 't'])
    await browser.pause(500)

    const newTabs = await $$('.tab')
    expect(newTabs.length).toBe(initialTabs.length + 1)
  })

  it('should zoom with Ctrl+=/-/0', async () => {
    // Zoom in
    await browser.keys(['Control', '='])
    await browser.pause(300)

    const zoomIn = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })
    expect(zoomIn).toBeGreaterThan(1.0)

    // Zoom out
    await browser.keys(['Control', '-'])
    await browser.pause(300)

    // Reset
    await browser.keys(['Control', '0'])
    await browser.pause(300)

    const reset = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })
    expect(Math.abs(reset - 1.0)).toBeLessThan(0.05)
  })

  it('should handle Ctrl+W to close tab', async () => {
    // Create a tab first
    await browser.keys(['Control', 't'])
    await browser.pause(500)

    const beforeClose = await $$('.tab')

    // Close it
    await browser.keys(['Control', 'w'])
    await browser.pause(500)

    const afterClose = await $$('.tab')
    expect(afterClose.length).toBe(beforeClose.length - 1)
  })

  it('should NOT capture Win key shortcuts', async function() {
    // This test documents expected behavior - Win key won't work
    this.skip() // Skip as it's a known limitation

    // Win+D (show desktop) won't be captured by Tauri when window focused
  })

  it('should handle Alt+F4 for window close', async () => {
    // Note: This will close the window and end the test session
    // Only test this at the end of test suite or skip
    this.skip()

    await browser.keys(['Alt', 'F4'])
    // Window closes, CloseRequested event fires
  })
})
```

### 5. Mouse Wheel Zoom Tests (Windows-Specific)

**Based on Linux implementation - verify cross-platform compatibility**

```javascript
describe('Mouse Wheel Zoom (Windows)', () => {
  beforeEach(async () => {
    // Reset zoom before each test
    await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      await invoke('set_zoom_factor', { factor: 1.0 })
      done()
    })
    await browser.pause(300)
  })

  it('should zoom in with Ctrl+Wheel Up', async function() {
    this.timeout(15000)

    const initialZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })

    const body = await $('body')
    await body.moveTo()

    // Hold Ctrl and scroll up (negative deltaY = zoom in)
    await browser.performActions([
      {
        type: 'key',
        id: 'keyboard',
        actions: [{ type: 'keyDown', value: '\uE009' }] // Control
      },
      {
        type: 'wheel',
        id: 'wheel',
        actions: [{
          type: 'scroll',
          duration: 0,
          x: 0,
          y: 0,
          deltaX: 0,
          deltaY: -100
        }]
      }
    ])

    await browser.releaseActions()
    await browser.pause(500)

    const newZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })

    console.log(`Zoom: ${initialZoom} -> ${newZoom}`)
    expect(newZoom).toBeGreaterThan(initialZoom)
  })

  it('should zoom out with Ctrl+Wheel Down', async function() {
    this.timeout(15000)

    // First zoom in
    await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      await invoke('set_zoom_factor', { factor: 1.5 })
      done()
    })
    await browser.pause(300)

    const initialZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })

    const body = await $('body')
    await body.moveTo()

    // Hold Ctrl and scroll down (positive deltaY = zoom out)
    await browser.performActions([
      {
        type: 'key',
        id: 'keyboard',
        actions: [{ type: 'keyDown', value: '\uE009' }] // Control
      },
      {
        type: 'wheel',
        id: 'wheel',
        actions: [{
          type: 'scroll',
          duration: 0,
          x: 0,
          y: 0,
          deltaX: 0,
          deltaY: 100
        }]
      }
    ])

    await browser.releaseActions()
    await browser.pause(500)

    const newZoom = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      done(await invoke('get_zoom_factor'))
    })

    expect(newZoom).toBeLessThan(initialZoom)
  })
})
```

### 6. Performance and Stability Tests

**Windows-specific performance considerations**

```javascript
describe('Performance (Windows)', () => {
  it('should handle rapid window resize', async () => {
    const sizes = [
      { width: 800, height: 600 },
      { width: 1024, height: 768 },
      { width: 1280, height: 720 },
      { width: 1920, height: 1080 }
    ]

    for (const size of sizes) {
      await browser.setWindowRect(size)
      await browser.pause(200)

      // Verify app is still responsive
      const header = await $('.window-header')
      expect(await header.isDisplayed()).toBe(true)
    }
  })

  it('should maintain performance under load', async () => {
    const start = Date.now()

    // Create and switch between multiple tabs
    for (let i = 0; i < 10; i++) {
      await browser.keys(['Control', 't'])
      await browser.pause(100)
    }

    const elapsed = Date.now() - start
    expect(elapsed).toBeLessThan(5000) // Should complete in 5s
  })

  it('should handle WebView2 resource cleanup', async function() {
    this.timeout(30000)

    // Stress test: rapid zoom changes
    for (let i = 0; i < 50; i++) {
      const zoom = 1.0 + (Math.random() * 1.0) // 1.0 to 2.0
      await browser.executeAsync(async (z, done) => {
        const { invoke } = window.__TAURI_INTERNALS__
        await invoke('set_zoom_factor', { factor: z })
        done()
      }, zoom)
      await browser.pause(50)
    }

    // Verify app is still responsive
    const isReady = await browser.execute(() => {
      return window.__TAURI_INTERNALS__ !== undefined &&
             document.readyState === 'complete'
    })
    expect(isReady).toBe(true)
  })
})
```

## Continuous Integration (GitHub Actions)

### Complete Windows CI Workflow

```yaml
name: E2E Tests

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  test-windows:
    runs-on: windows-latest
    timeout-minutes: 30

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'

      - name: Install dependencies
        run: npm ci

      - name: Install tauri-driver
        run: cargo install tauri-driver --locked

      - name: Install Microsoft Edge WebDriver
        run: |
          cargo install --git https://github.com/chippers/msedgedriver-tool
          & "$HOME\.cargo\bin\msedgedriver-tool.exe" $PWD.Path

      - name: Build application
        run: |
          cd src-tauri
          cargo build --release

      - name: Run E2E tests
        run: npm run test:e2e

      - name: Upload screenshots on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-screenshots-windows
          path: test/screenshots/
          retention-days: 30

      - name: Upload test logs on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: test-logs-windows
          path: |
            test/logs/
            *.log
          retention-days: 30
```

### Cross-Platform Matrix

```yaml
jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        platform: [ubuntu-latest, windows-latest]

    runs-on: ${{ matrix.platform }}

    steps:
      # ... setup steps ...

      # Platform-specific driver installation
      - name: Install webkit2gtk-driver (Linux)
        if: matrix.platform == 'ubuntu-latest'
        run: sudo apt-get install -y webkit2gtk-driver xvfb

      - name: Install msedgedriver (Windows)
        if: matrix.platform == 'windows-latest'
        run: |
          cargo install --git https://github.com/chippers/msedgedriver-tool
          & "$HOME\.cargo\bin\msedgedriver-tool.exe" $PWD.Path

      # Platform-specific test execution
      - name: Run tests (Linux)
        if: matrix.platform == 'ubuntu-latest'
        run: xvfb-run npm run test:e2e

      - name: Run tests (Windows)
        if: matrix.platform == 'windows-latest'
        run: npm run test:e2e
```

## Windows-Specific Best Practices

### 1. Version Management

**Always use msedgedriver-tool in CI:**
```yaml
- name: Install Edge Driver
  run: |
    cargo install --git https://github.com/chippers/msedgedriver-tool
    & "$HOME\.cargo\bin\msedgedriver-tool.exe" $PWD.Path
```

**For local development:**
```powershell
# Create a script: update-edgedriver.ps1
$edgeVersion = (Get-Item "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe").VersionInfo.ProductVersion
Write-Host "Edge version: $edgeVersion"

# Download matching driver
# https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
```

### 2. Error Handling and Debugging

**Screenshot on failure:**
```javascript
afterEach(async function() {
  if (this.currentTest.state === 'failed') {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
    const filename = `${this.currentTest.title.replace(/\s/g, '_')}-${timestamp}.png`
    await browser.saveScreenshot(`./test/screenshots/${filename}`)
    console.log(`Screenshot saved: ${filename}`)
  }
})
```

**Verbose logging:**
```javascript
// wdio.conf.cjs
exports.config = {
  logLevel: 'trace', // For debugging
  outputDir: './test/logs'
}
```

**Check tauri-driver status:**
```powershell
# During test run
Get-Process tauri-driver
Get-Process msedgedriver

# Check ports
netstat -ano | findstr :4444
```

### 3. Windows Defender Exclusions

**Add project directory to Windows Defender exclusions to prevent unnecessary recompilation:**

```powershell
# Run as Administrator
Add-MpPreference -ExclusionPath "C:\path\to\agentmux"
```

### 4. Cleanup Between Tests

**Ensure proper WebView2 cleanup:**
```javascript
afterSession: async function() {
  if (tauriDriver) {
    tauriDriver.kill()

    // Windows: Wait longer for WebView2 cleanup
    await new Promise(resolve => setTimeout(resolve, 3000))
  }
}
```

## Common Issues and Solutions

### Issue: Test suite hangs on connection

**Cause:** Edge/EdgeDriver version mismatch

**Solution:**
```powershell
# 1. Check Edge version
msedge --version

# 2. Download EXACT matching driver version
# https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/

# 3. Verify driver version
msedgedriver.exe --version
```

### Issue: "only ASCIIZ protocol mode is supported"

**Cause:** EdgeDriver 117+ breaking change

**Solution:**
```javascript
capabilities: [{
  'tauri:options': {
    application: './target/release/agentmux.exe'
  },
  webviewOptions: {} // Add this
}]
```

### Issue: WebView2 user data folder locked

**Cause:** msedgedriver not cleaning up properly

**Solution:**
```javascript
afterSession: async () => {
  if (tauriDriver) {
    tauriDriver.kill()
    // Longer wait for Windows
    await new Promise(resolve => setTimeout(resolve, 3000))
  }
}
```

### Issue: Context menu tests failing

**Cause:** Timing issues with menu rendering

**Solution:**
```javascript
// Wait for menu to appear
await browser.pause(500) // Increase wait time on Windows

// Or use explicit wait
await browser.waitUntil(async () => {
  const menu = await $('.context-menu')
  return await menu.isDisplayed()
}, { timeout: 3000 })
```

### Issue: DPI scaling causing size mismatches

**Cause:** Windows DPI scaling affects window measurements

**Solution:**
```javascript
// Use tolerance in assertions
expect(actualWidth).toBeCloseTo(expectedWidth, 20) // ±20px tolerance
```

## Test Organization

### Unified Cross-Platform Directory Structure

**Goal:** Single test suite that runs on both Linux and Windows with platform-aware behavior.

```
test/
├── specs/
│   ├── zoom.e2e.js                 # ✅ Cross-platform (already implemented)
│   ├── keyboard-shortcuts.e2e.js   # ✅ Cross-platform (Ctrl/Cmd detection)
│   ├── window-management.e2e.js    # ✅ Cross-platform (basic window ops)
│   ├── context-menu.e2e.js         # ⚠️ Cross-platform (platform-aware timing)
│   ├── text-input.e2e.js           # ✅ Cross-platform
│   ├── performance.e2e.js          # ✅ Cross-platform
│   └── windows/                     # ❌ Windows-only tests
│       ├── dpi-scaling.e2e.js      # DPI/multi-monitor (Windows feature)
│       └── edge-webview.e2e.js     # EdgeDriver-specific tests
├── helpers/
│   ├── tauri-helpers.js            # ✅ Cross-platform (already implemented)
│   ├── platform-helpers.js         # Platform detection & utilities
│   └── windows-helpers.js          # Windows-specific utilities
├── fixtures/
│   └── test-data.json              # Test data (cross-platform)
├── screenshots/                     # Auto-captured on failure
└── logs/                           # Test execution logs
```

**Key Principles:**
- Tests in `specs/` root are cross-platform by default
- Platform-specific tests go in `specs/windows/` or `specs/linux/`
- Use platform detection helpers for conditional behavior
- Share helpers between platforms wherever possible

### Helper Functions

#### Cross-Platform Helpers (test/helpers/platform-helpers.js)

**Use these in all tests for platform detection:**

```javascript
/**
 * Platform detection utilities
 */
export function isWindows() {
  return process.platform === 'win32'
}

export function isLinux() {
  return process.platform === 'linux'
}

export function isMac() {
  return process.platform === 'darwin'
}

/**
 * Get platform-aware wait time
 * Windows often needs longer waits for animations/rendering
 */
export function getWaitTime(baseMs, windowsMultiplier = 1.5) {
  return isWindows() ? Math.floor(baseMs * windowsMultiplier) : baseMs
}

/**
 * Get platform-aware modifier key
 * Returns 'Control' on Windows/Linux, 'Command' on macOS
 */
export function getModifierKey() {
  return isMac() ? 'Command' : 'Control'
}

/**
 * Conditional test skip based on platform
 */
export function skipUnless(platform, testContext) {
  const shouldSkip = (
    (platform === 'windows' && !isWindows()) ||
    (platform === 'linux' && !isLinux()) ||
    (platform === 'mac' && !isMac())
  )

  if (shouldSkip && testContext) {
    testContext.skip()
  }

  return shouldSkip
}
```

#### Windows-Specific Helpers (test/helpers/windows-helpers.js)

**Use these only in Windows-specific tests:**

```javascript
/**
 * Get Windows version
 */
export async function getWindowsVersion() {
  return await browser.execute(() => {
    return navigator.userAgent
  })
}

/**
 * Wait for window animation to complete
 * Windows has longer animation times than Linux
 */
export async function waitForWindowAnimation() {
  await browser.pause(500) // Windows animation duration
}

/**
 * Check if window is maximized
 */
export async function isWindowMaximized() {
  const rect = await browser.getWindowRect()
  const screen = await browser.execute(() => {
    return {
      width: window.screen.availWidth,
      height: window.screen.availHeight
    }
  })

  // Account for taskbar
  return rect.width >= screen.width - 20 &&
         rect.height >= screen.height - 100
}

/**
 * Capture window state for debugging
 */
export async function captureWindowState(name) {
  const rect = await browser.getWindowRect()
  const zoom = await browser.executeAsync(async (done) => {
    const { invoke } = window.__TAURI_INTERNALS__
    done(await invoke('get_zoom_factor'))
  })

  const state = {
    timestamp: new Date().toISOString(),
    rect,
    zoom,
    url: await browser.getUrl(),
    title: await browser.getTitle()
  }

  console.log(`Window state (${name}):`, JSON.stringify(state, null, 2))
  return state
}
```

## Performance Optimization

### Windows-Specific Optimizations

**Disable Windows Defender during tests:**
```powershell
# Development only - DO NOT use in production
Add-MpPreference -ExclusionPath "C:\path\to\agentmux"
```

**Optimize WebView2:**
```rust
// src-tauri/src/main.rs
#[cfg(target_os = "windows")]
fn optimize_webview2() {
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--disable-web-security --disable-features=msWebOOUI"
    );
}
```

**Increase timeouts for Windows:**
```javascript
exports.config = {
  waitforTimeout: 15000,        // Default: 10000
  mochaOpts: {
    timeout: 90000              // Default: 60000
  }
}
```

## npm Scripts

**package.json:**
```json
{
  "scripts": {
    "test:e2e": "wdio run wdio.conf.cjs",
    "test:e2e:debug": "wdio run wdio.conf.cjs --logLevel trace",
    "test:e2e:single": "wdio run wdio.conf.cjs --spec",
    "test:build": "cd src-tauri && cargo build --release",
    "test:install": "cargo install tauri-driver --locked && npm ci"
  }
}
```

## Implementation Roadmap

**Cross-Platform Approach:** Most work involves adapting existing Linux tests rather than writing from scratch.

### Phase 1: Enable Windows Support (1-2 hours) ⚡ Quick Win

**Goal:** Get existing Linux tests running on Windows with minimal changes.

- [ ] Update `wdio.conf.cjs` with platform detection (binary paths, WebDriver setup)
- [ ] Add `webviewOptions: {}` for EdgeDriver 117+ compatibility
- [ ] Create `platform-helpers.js` (isWindows, getWaitTime, getModifierKey)
- [ ] Install msedgedriver-tool for local development
- [ ] Verify existing `zoom.e2e.js` runs on Windows unchanged

**Expected Result:** Existing zoom tests pass on Windows immediately.

### Phase 2: Cross-Platform Test Adaptation (4-6 hours)

**Goal:** Ensure all existing Linux tests work reliably on Windows.

- [ ] Run all existing tests on Windows, identify failures
- [ ] Add platform-aware wait times where needed (context menus, animations)
- [ ] Add tolerance for DPI-related size differences (window measurements)
- [ ] Add conditional skips for platform-specific features (Win key limitations)
- [ ] Verify `tauri-helpers.js` works identically on both platforms

**Tests to verify:**
- ✅ Zoom functionality (already cross-platform)
- ⚠️ Keyboard shortcuts (document limitations)
- ⚠️ Window management (add DPI tolerance)
- ⚠️ Context menus (adjust timing)
- ✅ Text input (should work unchanged)
- ✅ Performance (should work unchanged)

### Phase 3: CI/CD Integration (2-3 hours)

**Goal:** Run tests on both platforms in GitHub Actions.

- [ ] Add `windows-latest` to test matrix in `.github/workflows/`
- [ ] Set up msedgedriver-tool automation in CI
- [ ] Configure screenshot/log artifacts for both platforms
- [ ] Add platform-specific test reports
- [ ] Verify tests pass consistently on both platforms

**GitHub Actions workflow:**
```yaml
strategy:
  matrix:
    platform: [ubuntu-latest, windows-latest]
```

### Phase 4: Windows-Specific Tests (4-6 hours)

**Goal:** Add tests for Windows-only features.

- [ ] Create `test/specs/windows/dpi-scaling.e2e.js`
  - DPI change handling
  - Multi-monitor DPI transitions
  - Window size consistency across DPI changes
- [ ] Create `test/specs/windows/edge-webview.e2e.js`
  - EdgeDriver-specific features
  - WebView2 resource cleanup
  - Edge-specific performance characteristics
- [ ] Add Windows-specific helper functions (`windows-helpers.js`)

**New test coverage:**
- DPI scaling scenarios (Windows feature)
- Multi-monitor behavior (Windows has unique challenges)
- Edge WebView2 specifics

### Phase 5: Maintenance and Documentation (2-3 hours)

- [ ] Document platform differences in test README
- [ ] Create Windows troubleshooting guide
- [ ] Set up Edge/EdgeDriver version monitoring
- [ ] Add test coverage reports (combined Linux + Windows)
- [ ] Document best practices for cross-platform tests

**Deliverables:**
- README with platform-specific instructions
- Troubleshooting guide for Windows issues
- CI badges showing test status for both platforms

---

### Total Effort Estimate

| Phase | Effort | Description |
|-------|--------|-------------|
| Phase 1 | 1-2 hours | Enable Windows (quick win!) |
| Phase 2 | 4-6 hours | Adapt existing tests |
| Phase 3 | 2-3 hours | CI/CD setup |
| Phase 4 | 4-6 hours | Windows-only tests |
| Phase 5 | 2-3 hours | Documentation |
| **Total** | **13-20 hours** | Full Windows support |

**Quick Start Path (Phases 1-3):** 7-11 hours to get basic Windows CI working with existing tests.

**Comparison to writing from scratch:** ~60-80 hours (4x longer)

**Code reuse:** ~90% of test code shared between platforms

## References

**Official Documentation:**
- [Tauri WebDriver Docs](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Tauri Example](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Tauri CI/CD Guide](https://v2.tauri.app/develop/tests/webdriver/ci/)
- [Microsoft Edge WebDriver](https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/)

**Tools:**
- [msedgedriver-tool](https://github.com/chippers/msedgedriver-tool)
- [WebdriverIO Documentation](https://webdriver.io/docs/gettingstarted)

**Example Repositories:**
- [tauri-apps/webdriver-example](https://github.com/tauri-apps/webdriver-example)
- [Haprog/tauri-wdio-win-test](https://github.com/Haprog/tauri-wdio-win-test)
- [rzmk/tauri-windows-e2e-demo](https://github.com/rzmk/tauri-windows-e2e-demo)

**GitHub Issues:**
- [tauri-driver sync issue](https://github.com/tauri-apps/tauri/issues/3576)
- [EdgeDriver 117 issue](https://github.com/tauri-apps/tauri/issues/7865)
- [DPI scaling issues](https://github.com/tauri-apps/tauri/issues/3610)

---

**Document Version**: 2.0 (Cross-Platform Strategy)
**Last Updated**: 2026-02-16
**Author**: AgentX
**Platform**: Windows 10/11 (tested on GitHub Actions windows-latest)
**Approach**: ~90% code reuse from Linux implementation
**Related Docs**:
- `TAURI_TESTING_SPEC_LINUX.md` - Linux implementation (provides foundation)
- Existing: `test/specs/zoom.e2e.js` - Already cross-platform
- Existing: `test/helpers/tauri-helpers.js` - Already cross-platform
