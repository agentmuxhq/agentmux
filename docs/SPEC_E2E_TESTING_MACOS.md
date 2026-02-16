# E2E Testing on macOS

## Overview

macOS E2E tests run the frontend against a Vite dev server with mocked Tauri IPC. This catches UI regressions (disappearing buttons, broken layouts, CSS issues) without needing `tauri-driver`, which has no macOS support.

## Architecture

| Layer | Config | Platforms | What it Tests |
|-------|--------|-----------|---------------|
| Full E2E | `wdio.conf.cjs` | Linux, Windows | Real app with backend via `tauri-driver` |
| Frontend E2E | `wdio.macos.conf.cjs` | macOS (+ any platform) | UI against Vite dev server with mocked IPC |

Both layers share the same `test/specs/` and `test/helpers/`.

## File Structure

```
wdio.conf.cjs                      # Linux/Windows: tauri-driver
wdio.macos.conf.cjs                # macOS: devtools protocol + mocked IPC
test/
  helpers/
    tauri-helpers.js                # byTestId(), waitForZoomChange(), etc.
    platform-helpers.js             # Platform detection utilities
  specs/
    zoom.e2e.js                     # Zoom keyboard/mouse/API tests
    window-controls.e2e.js          # Window header, agentmux button, widgets
    layout.e2e.js                   # Block panes and headers
  screenshots/                      # Failure screenshots
```

## Running Tests

### macOS (mocked IPC)

```bash
# Via npm script (starts Vite automatically via Taskfile):
npm run test:e2e:macos

# Or manually:
npx vite --config vite.config.tauri.ts &
npx wdio run wdio.macos.conf.cjs
kill %1

# Via Taskfile:
task test:e2e:macos
```

### Linux/Windows (real backend)

```bash
npm run test:e2e
```

## Test Selectors

Tests use `data-testid` attributes for stable selectors that survive CSS class renames:

| `data-testid` | Component | File |
|---------------|-----------|------|
| `window-controls` | Outer div | `window-controls.tsx` |
| `new-window-btn` | New window button | `window-controls.tsx` |
| `window-header` | Header bar | `window-header.tsx` |
| `window-minimize-btn` | Minimize | `system-status.tsx` |
| `window-maximize-btn` | Maximize | `system-status.tsx` |
| `window-close-btn` | Close | `system-status.tsx` |
| `action-widgets` | Widget bar | `action-widgets.tsx` |
| `block-header` | Block frame header | `blockframe.tsx` |

Helper function:
```javascript
import { byTestId } from '../helpers/tauri-helpers.js'
const btn = await $(byTestId('new-window-btn'))
```

## Mocked IPC

`wdio.macos.conf.cjs` injects `window.__TAURI_INTERNALS__` with mock responses for all commands used during app initialization. The mock includes mutable zoom state so zoom tests work against the mock.

Key mocked commands: `get_auth_key`, `get_is_dev`, `get_platform`, `get_zoom_factor`, `set_zoom_factor`, `get_backend_endpoints`, `get_about_modal_details`, and window management commands.

## What This Catches

The agentmux button disappearing (a real bug we encountered) is caught by `window-controls.e2e.js`:

```
FAIL should show the agentmux button
  Element [data-testid="new-window-btn"] did not become ready
```

## Future: Full E2E on macOS

When a macOS WebDriver solution ships for WKWebView:

1. Update `wdio.macos.conf.cjs` to use the real driver
2. Remove the IPC mock layer
3. All `test/specs/` run against the real app with zero test changes

## References

- [Tauri WebDriver Docs](https://v2.tauri.app/develop/tests/webdriver/)
- [WDIO DevTools Service](https://webdriver.io/docs/devtools-service/)
- [macOS WebDriver issue #7068](https://github.com/tauri-apps/tauri/issues/7068)
- [Existing Linux spec](./TAURI_TESTING_SPEC_LINUX.md)
- [Windows testing spec](./TAURI_TESTING_SPEC_WINDOWS.md)
