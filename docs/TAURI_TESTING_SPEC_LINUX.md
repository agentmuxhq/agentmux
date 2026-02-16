# Tauri Testing Specification for Linux

## Overview

This document outlines best practices and implementation guidelines for automated end-to-end (E2E) testing of AgentMux on Linux using the Tauri WebDriver framework.

## Testing Stack

### Core Components

- **tauri-driver**: Cross-platform WebDriver wrapper for Tauri applications
- **WebdriverIO (WDIO)**: Node.js test automation framework with excellent Tauri support
- **WebKitWebDriver**: Native Linux WebDriver server (webkit2gtk-driver package)
- **Xvfb**: Virtual framebuffer for headless testing in CI environments

### Alternative Options

- **Selenium**: Full-featured testing framework (more heavyweight than WebdriverIO)
- **Playwright**: Modern testing framework (experimental Tauri support)

## Setup

### Dependencies

#### System Packages (Debian/Ubuntu)

```bash
# WebDriver for webkit2gtk (required on Linux)
sudo apt-get install webkit2gtk-driver

# Virtual display for headless testing (CI/CD)
sudo apt-get install xvfb

# Additional dependencies
sudo apt-get install libwebkit2gtk-4.1-dev
```

#### Node.js Dependencies

```json
{
  "devDependencies": {
    "@wdio/cli": "^8.0.0",
    "@wdio/local-runner": "^8.0.0",
    "@wdio/mocha-framework": "^8.0.0",
    "@wdio/spec-reporter": "^8.0.0",
    "webdriverio": "^8.0.0"
  }
}
```

#### Rust Dependencies (Optional)

```toml
[dev-dependencies]
tauri-driver = "0.1"
```

### WebdriverIO Configuration

Create `wdio.conf.js`:

```javascript
const os = require('os')
const path = require('path')
const { spawn, spawnSync } = require('child_process')

// Keep track of the tauri-driver process
let tauriDriver

exports.config = {
  runner: 'local',
  specs: ['./test/specs/**/*.e2e.js'],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: '../../src-tauri/target/release/agentmux'
      }
    }
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    timeout: 60000
  },

  // Ensure tauri-driver is running before tests start
  onPrepare: () => {
    tauriDriver = spawn(
      path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver'),
      [],
      { stdio: [null, process.stdout, process.stderr] }
    )
  },

  // Clean up tauri-driver after tests complete
  onComplete: () => {
    tauriDriver.kill()
  }
}
```

## Test Categories

### 1. Window Management Tests

Test window drag, resize, minimize, maximize, and close operations.

```javascript
describe('Window Management', () => {
  it('should drag window by custom drag region', async () => {
    const dragRegion = await $('[data-tauri-drag-region]')
    const initialPos = await browser.getWindowRect()

    await dragRegion.moveTo()
    await browser.performActions([
      {
        type: 'pointer',
        id: 'mouse',
        parameters: { pointerType: 'mouse' },
        actions: [
          { type: 'pointerMove', duration: 0, x: 0, y: 0 },
          { type: 'pointerDown', button: 0 },
          { type: 'pointerMove', duration: 100, x: 100, y: 100 },
          { type: 'pointerUp', button: 0 }
        ]
      }
    ])

    const newPos = await browser.getWindowRect()
    expect(newPos.x).not.toBe(initialPos.x)
    expect(newPos.y).not.toBe(initialPos.y)
  })

  it('should minimize and restore window', async () => {
    await browser.minimizeWindow()
    await browser.pause(500)

    await browser.maximizeWindow()
    const rect = await browser.getWindowRect()
    expect(rect.width).toBeGreaterThan(800)
  })
})
```

### 2. Text Input Tests

Test text input in terminal panes, search boxes, and forms.

```javascript
describe('Text Input', () => {
  it('should accept keyboard input in terminal', async () => {
    const terminal = await $('.terminal')
    await terminal.click()
    await browser.keys(['echo', ' ', 'test', 'Enter'])
    await browser.pause(500)

    const output = await $('.terminal-output').getText()
    expect(output).toContain('test')
  })

  it('should handle special keys', async () => {
    const input = await $('input[type="text"]')
    await input.setValue('Hello')
    await browser.keys(['Control', 'a'])
    await browser.keys('Backspace')

    const value = await input.getValue()
    expect(value).toBe('')
  })

  it('should support multi-line input', async () => {
    const textarea = await $('textarea')
    await textarea.setValue('Line 1')
    await browser.keys(['Shift', 'Enter'])
    await browser.keys('Line 2')

    const value = await textarea.getValue()
    expect(value).toBe('Line 1\nLine 2')
  })
})
```

### 3. Mouse Context Menu Tests

Test right-click context menus and menu item selection.

```javascript
describe('Context Menus', () => {
  it('should open terminal context menu', async () => {
    const terminal = await $('.terminal-pane')
    await terminal.click({ button: 'right' })
    await browser.pause(300)

    const contextMenu = await $('.context-menu')
    expect(await contextMenu.isDisplayed()).toBe(true)
  })

  it('should select menu item', async () => {
    const element = await $('.terminal-pane')
    await element.click({ button: 'right' })

    const copyItem = await $('=Copy')
    await copyItem.click()

    // Verify action completed
    const clipboard = await browser.execute(() => {
      return navigator.clipboard.readText()
    })
    expect(clipboard.length).toBeGreaterThan(0)
  })

  it('should close menu on Escape', async () => {
    const element = await $('.terminal-pane')
    await element.click({ button: 'right' })

    const menu = await $('.context-menu')
    expect(await menu.isDisplayed()).toBe(true)

    await browser.keys('Escape')
    expect(await menu.isDisplayed()).toBe(false)
  })
})
```

### 4. Scroll and Zoom Tests

Test scroll operations and zoom functionality (critical for our Linux zoom fix).

```javascript
describe('Scroll and Zoom', () => {
  it('should scroll terminal with mouse wheel', async () => {
    const terminal = await $('.terminal')
    const initialScroll = await browser.execute(() => {
      return document.querySelector('.terminal').scrollTop
    })

    await terminal.moveTo()
    await browser.performActions([
      {
        type: 'wheel',
        id: 'wheel',
        actions: [
          { type: 'scroll', duration: 0, x: 0, y: 0, deltaX: 0, deltaY: 100 }
        ]
      }
    ])

    const newScroll = await browser.execute(() => {
      return document.querySelector('.terminal').scrollTop
    })
    expect(newScroll).toBeGreaterThan(initialScroll)
  })

  it('should zoom with Ctrl+Wheel', async () => {
    const body = await $('body')
    const initialZoom = await browser.execute(() => {
      return parseFloat(getComputedStyle(document.body).zoom) || 1.0
    })

    await body.moveTo()
    await browser.performActions([
      {
        type: 'key',
        id: 'keyboard',
        actions: [
          { type: 'keyDown', value: '\uE009' } // Control key
        ]
      },
      {
        type: 'wheel',
        id: 'wheel',
        actions: [
          { type: 'scroll', duration: 0, x: 0, y: 0, deltaX: 0, deltaY: -100 }
        ]
      }
    ])
    await browser.releaseActions()
    await browser.pause(300)

    const newZoom = await browser.execute(() => {
      return parseFloat(getComputedStyle(document.body).zoom) || 1.0
    })
    expect(newZoom).toBeGreaterThan(initialZoom)
  })

  it('should zoom with keyboard shortcuts', async () => {
    // Zoom in
    await browser.keys(['Control', '='])
    await browser.pause(200)

    const zoomIn = await browser.execute(() => {
      return parseFloat(getComputedStyle(document.body).zoom) || 1.0
    })
    expect(zoomIn).toBeGreaterThan(1.0)

    // Zoom out
    await browser.keys(['Control', '-'])
    await browser.pause(200)

    // Reset
    await browser.keys(['Control', '0'])
    await browser.pause(200)

    const reset = await browser.execute(() => {
      return parseFloat(getComputedStyle(document.body).zoom) || 1.0
    })
    expect(reset).toBeCloseTo(1.0, 1)
  })
})
```

### 5. Keyboard Shortcuts Tests

Test global and local keyboard shortcuts.

```javascript
describe('Keyboard Shortcuts', () => {
  it('should create new terminal with Ctrl+T', async () => {
    const initialTerminals = await $$('.terminal-pane')
    await browser.keys(['Control', 't'])
    await browser.pause(500)

    const newTerminals = await $$('.terminal-pane')
    expect(newTerminals.length).toBe(initialTerminals.length + 1)
  })

  it('should switch tabs with Ctrl+Tab', async () => {
    const firstTab = await $('.tab.active')
    const firstTabId = await firstTab.getAttribute('data-tab-id')

    await browser.keys(['Control', 'Tab'])
    await browser.pause(300)

    const activeTab = await $('.tab.active')
    const activeTabId = await activeTab.getAttribute('data-tab-id')
    expect(activeTabId).not.toBe(firstTabId)
  })
})
```

### 6. Multi-Window Tests

Test multiple window instances and inter-window communication.

```javascript
describe('Multi-Window', () => {
  it('should open new window', async () => {
    const windows = await browser.getWindowHandles()
    const initialCount = windows.length

    // Trigger new window (via menu or shortcut)
    await browser.keys(['Control', 'Shift', 'n'])
    await browser.pause(1000)

    const newWindows = await browser.getWindowHandles()
    expect(newWindows.length).toBe(initialCount + 1)
  })

  it('should switch between windows', async () => {
    const windows = await browser.getWindowHandles()

    await browser.switchToWindow(windows[0])
    const title1 = await browser.getTitle()

    await browser.switchToWindow(windows[1])
    const title2 = await browser.getTitle()

    expect(title1).toBe('AgentMux')
    expect(title2).toBe('AgentMux')
  })
})
```

### 7. IPC Communication Tests

Test frontend-backend IPC calls.

```javascript
describe('IPC Communication', () => {
  it('should invoke Tauri commands', async () => {
    const version = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      const result = await invoke('get_zoom_factor')
      done(result)
    })

    expect(version).toBeGreaterThan(0)
  })

  it('should handle command errors gracefully', async () => {
    const result = await browser.executeAsync(async (done) => {
      const { invoke } = window.__TAURI_INTERNALS__
      try {
        await invoke('non_existent_command')
        done({ error: false })
      } catch (e) {
        done({ error: true, message: e.message })
      }
    })

    expect(result.error).toBe(true)
  })
})
```

## Continuous Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test-linux:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y webkit2gtk-driver xvfb \
            libwebkit2gtk-4.1-dev build-essential curl wget \
            libssl-dev libgtk-3-dev libayatana-appindicator3-dev \
            librsvg2-dev

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install tauri-driver
        run: cargo install tauri-driver

      - name: Install dependencies
        run: npm ci

      - name: Build application
        run: npm run tauri build

      - name: Run E2E tests (headless)
        run: |
          xvfb-run --auto-servernum npm run test:e2e

      - name: Upload screenshots on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: screenshots
          path: ./test/screenshots/
```

## Mocking Tauri APIs

For unit tests running in Node.js without Tauri context:

```javascript
// test/setup.js
global.window.__TAURI_INTERNALS__ = {
  invoke: jest.fn(),
  listen: jest.fn()
}

global.window.api = {
  getZoomFactor: jest.fn(() => 1.0),
  setZoomFactor: jest.fn(),
  getPlatform: jest.fn(() => 'linux')
}
```

## Performance Testing

### Long-Running Stability Tests

```javascript
describe('Stability', () => {
  it('should handle 100 terminal operations without crash', async () => {
    for (let i = 0; i < 100; i++) {
      const terminal = await $('.terminal')
      await terminal.click()
      await browser.keys(['echo', ' ', `test${i}`, 'Enter'])
      await browser.pause(100)
    }

    const isRunning = await browser.execute(() => {
      return window.__TAURI_INTERNALS__ !== undefined
    })
    expect(isRunning).toBe(true)
  })

  it('should maintain performance under load', async () => {
    const start = Date.now()

    for (let i = 0; i < 50; i++) {
      await browser.keys(['Control', 't']) // New tab
      await browser.pause(50)
    }

    const elapsed = Date.now() - start
    expect(elapsed).toBeLessThan(10000) // Should complete in 10s
  })
})
```

## Test Organization

### Directory Structure

```
test/
├── specs/
│   ├── window.e2e.js
│   ├── input.e2e.js
│   ├── context-menu.e2e.js
│   ├── scroll-zoom.e2e.js
│   ├── shortcuts.e2e.js
│   └── stability.e2e.js
├── fixtures/
│   ├── test-commands.sh
│   └── sample-data.json
├── screenshots/
└── helpers/
    ├── tauri-helpers.js
    └── custom-commands.js
```

### Helper Functions

```javascript
// test/helpers/tauri-helpers.js
export async function getTauriVersion() {
  return await browser.executeAsync(async (done) => {
    const { invoke } = window.__TAURI_INTERNALS__
    const details = await invoke('get_about_modal_details')
    done(details.version)
  })
}

export async function waitForTerminalReady(selector = '.terminal') {
  await browser.waitUntil(
    async () => {
      const terminal = await $(selector)
      return await terminal.isDisplayed()
    },
    {
      timeout: 10000,
      timeoutMsg: 'Terminal did not become ready'
    }
  )
}

export async function captureScreenshot(name) {
  await browser.saveScreenshot(`./test/screenshots/${name}.png`)
}
```

## Best Practices

### 1. Test Isolation
- Each test should be independent
- Clean up state between tests
- Use `beforeEach` and `afterEach` hooks

### 2. Explicit Waits
- Always wait for elements to be ready
- Use `waitUntil` instead of fixed `pause`
- Set reasonable timeouts

### 3. Error Handling
- Capture screenshots on failure
- Log relevant state information
- Use try-catch for IPC calls

### 4. Platform-Specific Tests
```javascript
const isLinux = os.platform() === 'linux'

describe('Linux-specific tests', () => {
  if (!isLinux) {
    it.skip('skipped on non-Linux', () => {})
    return
  }

  it('should use WebKitGTK', async () => {
    // Linux-specific assertions
  })
})
```

### 5. Debugging Tips
- Enable DevTools in test builds: `TAURI_ENV_DEBUG=true`
- Use `browser.debug()` to pause and inspect
- Check tauri-driver logs for WebDriver issues
- Verify webkit2gtk-driver is running: `ps aux | grep webkit`

## Common Issues and Solutions

### Issue: Tests hang on Linux
**Solution**: Ensure xvfb is running and webkit2gtk-driver is installed

```bash
xvfb-run --auto-servernum npm run test:e2e
```

### Issue: Can't find elements
**Solution**: Use DevTools to verify selectors, wait for elements

```javascript
await browser.waitUntil(async () => {
  const el = await $(selector)
  return await el.isDisplayed()
})
```

### Issue: Mouse actions don't work
**Solution**: Ensure element is in viewport and use `moveTo()` before actions

```javascript
const element = await $(selector)
await element.scrollIntoView()
await element.moveTo()
```

### Issue: Zoom tests fail
**Solution**: Query actual zoom value from Tauri API, not DOM

```javascript
const zoom = await browser.executeAsync(async (done) => {
  const { invoke } = window.__TAURI_INTERNALS__
  const factor = await invoke('get_zoom_factor')
  done(factor)
})
```

## References

- [Tauri WebDriver Documentation](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Example for Tauri](https://v2.tauri.app/develop/tests/webdriver/example/webdriverio/)
- [Tauri Testing Discussion](https://github.com/tauri-apps/tauri/discussions/3768)
- [WebdriverIO API Documentation](https://webdriver.io/docs/api)
- [Tauri CI Testing Guide](https://v2.tauri.app/develop/tests/webdriver/ci/)
- [Testing Tauri with Selenium](https://wiprotechblogs.medium.com/automating-testing-of-tauri-app-with-selenium-1a58f64a6233)

## Implementation Roadmap

### Phase 1: Setup (Week 1)
- [ ] Install dependencies (tauri-driver, WebdriverIO)
- [ ] Configure wdio.conf.js
- [ ] Set up test directory structure
- [ ] Create helper functions

### Phase 2: Core Tests (Week 2-3)
- [ ] Window management tests
- [ ] Text input tests
- [ ] Context menu tests
- [ ] Keyboard shortcut tests

### Phase 3: Advanced Tests (Week 4)
- [ ] Scroll and zoom tests (priority for Linux zoom fix validation)
- [ ] Multi-window tests
- [ ] IPC communication tests
- [ ] Stability tests

### Phase 4: CI Integration (Week 5)
- [ ] Configure GitHub Actions
- [ ] Set up headless testing with xvfb
- [ ] Add screenshot capture on failure
- [ ] Create test reports

### Phase 5: Maintenance
- [ ] Document test failures and fixes
- [ ] Update tests for new features
- [ ] Review and refactor test suite
- [ ] Performance optimization

---

**Document Version**: 1.0
**Last Updated**: 2026-02-16
**Author**: AgentX
**Platform**: Linux (Ubuntu 24.04+ tested)
