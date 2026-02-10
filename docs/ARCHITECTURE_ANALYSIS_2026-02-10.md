# AgentMux Architecture Analysis & Modularization Proposal
## Deep Dive: Post Go-Sidecar Removal

**Date**: February 10, 2026
**Version**: 0.20.19 (in progress)
**Author**: AgentA
**Context**: Investigating grey screen issue after Go sidecar removal

---

## Executive Summary

AgentMux has successfully transitioned from a dual-process architecture (Electron + Go sidecar) to a single-process Tauri v2 application with an in-process Rust backend. However, a critical rendering issue has emerged: **the UI displays as grey despite successful initialization, RPC communication, and React rendering**.

This document provides:
1. Complete architectural teardown of current state
2. Analysis of rendering pipeline and potential failure points
3. Modularization recommendations
4. Hypothesis on the root cause

---

## Current Architecture Overview

### High-Level Structure

```
┌─────────────────────────────────────────────────────┐
│  AgentMux.exe (Tauri v2 Single Process)            │
│                                                      │
│  ┌────────────────────┐  ┌─────────────────────┐  │
│  │   Rust Backend     │  │  Frontend (WebView) │  │
│  │                    │  │                      │  │
│  │  • WaveStore (DB)  │  │  • React/TypeScript │  │
│  │  • FileStore (DB)  │  │  • Xterm.js         │  │
│  │  • RPC Router      │  │  • Jotai State      │  │
│  │  • PTY Controller  │  │  • Monaco Editor    │  │
│  │  • Config System   │  │                      │  │
│  │  • wsh IPC Server  │  │                      │  │
│  └────────────────────┘  └─────────────────────┘  │
│           ↕                        ↕                │
│    Tauri IPC Bridge                                │
│    (invoke_handler)                                │
└─────────────────────────────────────────────────────┘
              ↕
     ┌────────────────┐
     │  wsh (Go CLI)  │  ← External process
     │  Named Pipe IPC │
     └────────────────┘
```

### Key Components

#### 1. Rust Backend (`src-tauri/src/`)

**Entry Point**: `lib.rs`
- Initializes Tauri app with 15+ plugins
- Registers 50+ IPC command handlers
- Sets up app menu and system tray
- Initializes heartbeat monitoring
- Calls `rust_backend::initialize()`

**Backend Initialization**: `rust_backend.rs`
- Opens SQLite stores (WaveStore, FileStore)
- Loads configuration (themes, widgets, presets)
- Creates client/window/tab objects
- **CRITICAL**: Async spawns RPC router and wsh IPC server
- Stores state in `AppState` managed by Tauri

**RPC System**: `backend/rpc/`
- `engine.rs`: WshRpcEngine - core RPC executor
- `router.rs`: WshRouter - routes commands to engine
- Input/output channels for async message passing
- **ISSUE**: Uses `tauri::async_runtime::spawn` (fixed)

**IPC Server**: `backend/wsh_server.rs`
- Named pipe server on Windows (`\\.\pipe\agentmux-{pid}`)
- Accepts connections from wsh CLI
- Handles authentication
- **ISSUE**: Used `tokio::spawn` (fixed)

**Command Handlers**: `commands/`
- `platform.rs`: OS info, paths
- `backend.rs`: Endpoints, init opts, logging
- `rpc.rs`: **CRITICAL** - RPC bridge between frontend and backend
  - `rpc_request()`: Generic RPC calls
  - `service_request()`: Typed service calls (GetClientData, GetWindow, etc.)
  - Response format: `{"data": ..., "updates": []}`
  - **ADDED**: otype field to objects

#### 2. Frontend (`frontend/`)

**Bootstrap**: `tauri-bootstrap.ts`
- Initializes Tauri API
- Validates API methods
- Logs pre-load state
- Imports `wave.ts` (main app)
- Logs post-load state

**Main App**: `wave.ts`
- Calls `globalInitTauri()` from `tauri-init.ts`
- Fetches init opts via `getWaveInitOpts()`
- Makes service calls: GetClientData, GetWindow, GetWorkspace
- Initializes WPS (pub/sub) client
- Renders React root

**Rendering**: `app/app.tsx`
- Root component with global state providers
- WorkspaceView renders active workspace
- BlockFrame renders terminal panes

**Styling**: `app/app.scss`
```scss
body {
    background-color: var(--main-bg-color);  // rgb(34, 34, 34)
    // ... other styles
}
```

**Build**: Vite with Tailwind CSS v4
- Output: `dist/frontend/assets/index-{hash}.css`
- Confirmed body styles are bundled
- CSS variables defined inline in `index.html`

---

## Rendering Pipeline Analysis

### Expected Flow

1. **HTML Load** (`index.html`)
   - Parse HTML
   - Load CSS: `/assets/index-{hash}.css`
   - Define CSS variables inline in `<style>`
   - Load JS: `/assets/index-{hash}.js`

2. **CSS Application**
   - Apply `body { background-color: var(--main-bg-color); }`
   - Variable resolves to `rgb(34, 34, 34)` (dark grey/almost black)
   - **Expected Visual**: Dark grey background

3. **JavaScript Initialization**
   - Tauri API loads
   - Bootstrap script runs
   - Observers attach to body and #main

4. **React Render**
   - `wave.ts` initializes
   - Fetches client/window/workspace data
   - Body class changes: `init` → `init is-transparent` → `init`
   - During `is-transparent`: `background-color: transparent`
   - After: returns to `rgb(34, 34, 34)`
   - **Expected Visual**: Brief flash of transparency, then dark grey

5. **Content Render**
   - WorkspaceView renders
   - One child added to #main
   - "Wave First Render Done" logged

### Actual Behavior

**Visual**: Grey screen (lighter grey, not dark)
**Logs**: All steps complete successfully
**RPC**: All service calls return valid data
**DOM**: Body has correct class, #main has 1 child
**CSS**: File loaded, styles bundled correctly

### Failure Point Hypotheses

#### Hypothesis 1: CSS Variable Resolution Failure
**Symptoms**: Background shows default grey instead of `rgb(34, 34, 34)`
**Cause**: WebView2 doesn't resolve `var(--main-bg-color)` correctly
**Evidence**:
- ❌ CSS variables ARE defined in inline `<style>`
- ❌ Body style DOES use `var(--main-bg-color)`
- ❓ Computed value unknown (debug file not written)

**Test**: Add fallback color
```css
body {
    background-color: rgb(34, 34, 34); /* fallback */
    background-color: var(--main-bg-color);
}
```

#### Hypothesis 2: Style Priority/Specificity Issue
**Symptoms**: Another style overrides body background
**Cause**: Tailwind reset, global styles, or inline styles have higher specificity
**Evidence**:
- ✅ No inline `style=""` on body element
- ✅ `.is-transparent` only applied temporarily
- ❓ Tailwind reset might be overriding

**Test**: Add `!important`
```css
body {
    background-color: var(--main-bg-color) !important;
}
```

#### Hypothesis 3: Window Background Showing Through
**Symptoms**: Tauri window default background visible
**Cause**: Body/main not covering entire viewport
**Evidence**:
- ✅ Body has `width: 100vw; height: 100vh;`
- ✅ Window is not transparent (`transparent: false`)
- ❓ Computed width/height unknown

**Test**: Check if body is actually 100% viewport size

#### Hypothesis 4: Render Layer / Compositing Issue
**Symptoms**: Styles computed but not painted
**Cause**: WebView2 rendering engine bug, GPU compositing issue
**Evidence**:
- ✅ `transform: translateZ(0)` forces GPU layer
- ✅ `backface-visibility: hidden`
- ❓ Actual rendering state unknown

**Test**: Disable hardware acceleration

#### Hypothesis 5: CSS Load Order / FOUC
**Symptoms**: Flash of Unstyled Content extended
**Cause**: CSS loads after initial paint, styles not applied retroactively
**Evidence**:
- ✅ CSS linked in `<head>` with `<link rel="stylesheet">`
- ✅ No `defer` or `async` on stylesheet
- ✅ Variables defined inline (immediate)
- ❓ Actual load timing unknown

**Test**: Add preload hint, inline critical CSS

#### Hypothesis 6: React Overriding Body Styles
**Symptoms**: React app sets inline styles or classes that override
**Cause**: Component lifecycle, style injection
**Evidence**:
- ✅ Body class only has `init` or `init is-transparent`
- ✅ No React portal rendering to body
- ❓ App component tree unknown

**Test**: Check if any React component touches body

---

## Modularization Proposal

### Current Issues with Monolithic Structure

1. **Tight Coupling**: Frontend directly calls backend RPC via Tauri IPC
2. **Mixed Concerns**: `lib.rs` does setup, initialization, event handling
3. **State Sprawl**: AppState in multiple mutex-wrapped fields
4. **Hard to Test**: Can't test frontend without full Tauri app
5. **Debug Opacity**: Can't inspect intermediate state easily

### Proposed Modular Architecture

```
┌─────────────────────────────────────────────────────────┐
│  AgentMux Application                                    │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Presentation Layer (Frontend)                     │ │
│  │  • React Components (pure UI)                      │ │
│  │  • View Models (state + actions)                   │ │
│  │  • Adapters (IPC, WebSocket, localStorage)        │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Application Layer (Business Logic)                │ │
│  │  • Services (WorkspaceService, TabService)         │ │
│  │  • Use Cases (CreateTab, SpawnAgent, etc.)         │ │
│  │  • DTOs (Data Transfer Objects)                    │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Domain Layer (Core Models)                        │ │
│  │  • Entities (Client, Window, Workspace, Tab)       │ │
│  │  • Value Objects (BlockId, TabId, etc.)            │ │
│  │  • Domain Events                                   │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Infrastructure Layer (Platform)                   │ │
│  │  • Repositories (WaveStore, FileStore)             │ │
│  │  • IPC Server (wsh socket)                         │ │
│  │  • PTY Controller                                  │ │
│  │  • Config Loader                                   │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Refactoring Steps

#### Phase 1: Extract Domain Models
**Goal**: Pure Rust types with no dependencies on Tauri or storage

**Files**:
```
src-tauri/src/domain/
  ├── entities/
  │   ├── client.rs
  │   ├── window.rs
  │   ├── workspace.rs
  │   ├── tab.rs
  │   └── block.rs
  ├── value_objects/
  │   ├── ids.rs          // BlockId, TabId, etc.
  │   ├── layout.rs       // LayoutState
  │   └── config.rs       // Settings, Presets
  └── events.rs           // Domain events
```

**Benefits**:
- Can test business logic without Tauri
- Clear data contracts
- No serde/tauri dependencies in core

#### Phase 2: Create Service Layer
**Goal**: Encapsulate business logic in stateless services

**Files**:
```
src-tauri/src/services/
  ├── workspace_service.rs  // CRUD for workspaces
  ├── tab_service.rs         // CRUD for tabs
  ├── block_service.rs       // Terminal block management
  ├── agent_service.rs       // AI agent spawning
  └── config_service.rs      // Config read/write
```

**Example**:
```rust
pub struct WorkspaceService {
    store: Arc<WaveStore>,
}

impl WorkspaceService {
    pub fn get_workspace(&self, id: &str) -> Result<Workspace, Error> {
        // Business logic here
    }

    pub fn create_workspace(&self, name: &str) -> Result<Workspace, Error> {
        // Validation, defaults, etc.
    }
}
```

**Benefits**:
- Testable with mock stores
- Reusable across IPC, CLI, tests
- Clear API surface

#### Phase 3: Refactor IPC Commands
**Goal**: Thin command handlers that delegate to services

**Before**:
```rust
#[tauri::command]
pub fn service_request(data: Value, state: State<AppState>) -> Result<Value, String> {
    // 200 lines of logic mixed with data access
}
```

**After**:
```rust
#[tauri::command]
pub fn get_workspace(id: String, state: State<AppState>) -> Result<Workspace, String> {
    state.workspace_service.get_workspace(&id)
        .map_err(|e| e.to_string())
}
```

**Benefits**:
- Commands are just adapters
- Logic lives in services
- Can add GraphQL/gRPC later without duplicating logic

#### Phase 4: Separate Frontend State Management
**Goal**: Decouple frontend atoms from backend RPC

**Files**:
```
frontend/app/services/
  ├── workspace_service.ts   // Frontend service
  ├── tab_service.ts
  └── rpc_client.ts          // Typed RPC client

frontend/app/models/
  ├── workspace.ts           // TypeScript types
  ├── tab.ts
  └── block.ts
```

**Pattern**:
```typescript
// Service layer
class WorkspaceService {
    async getWorkspace(id: string): Promise<Workspace> {
        const result = await rpcClient.call("workspace.GetWorkspace", {id});
        return Workspace.fromJSON(result.data);
    }
}

// React component
const workspace = useWorkspace(workspaceId);  // Hook uses service
```

**Benefits**:
- Clear separation: UI ← Service ← RPC
- Can mock services for Storybook
- Type safety end-to-end

#### Phase 5: Extract Rendering Pipeline
**Goal**: Isolate rendering concerns from business logic

**Files**:
```
frontend/app/rendering/
  ├── theme_loader.ts         // Load and apply CSS variables
  ├── layout_engine.ts        // Calculate pane sizes
  └── style_debugger.ts       // Debug CSS issues
```

**Key Function**:
```typescript
// theme_loader.ts
export function loadTheme(theme: Theme): void {
    const root = document.documentElement;

    // Set CSS variables
    root.style.setProperty('--main-bg-color', theme.colors.background);
    root.style.setProperty('--main-text-color', theme.colors.text);

    // Force repaint
    document.body.offsetHeight;  // Trigger reflow

    // Verify application
    const computed = getComputedStyle(document.body);
    const actualBg = computed.backgroundColor;

    if (actualBg !== theme.colors.background) {
        console.error(`Theme not applied! Expected ${theme.colors.background}, got ${actualBg}`);
        // Fallback: apply inline styles
        document.body.style.backgroundColor = theme.colors.background;
    }
}
```

**Benefits**:
- Centralized theme logic
- Self-verifying (detects failures)
- Can add hot-reload

---

## Debugging Strategy with Modular Architecture

### 1. Isolated Component Testing

**Frontend Rendering Test**:
```typescript
// test/rendering.test.ts
describe('Theme Application', () => {
    it('applies dark theme to body', () => {
        loadTheme(DARK_THEME);
        const bg = getComputedStyle(document.body).backgroundColor;
        expect(bg).toBe('rgb(34, 34, 34)');
    });
});
```

**Rust Service Test**:
```rust
#[test]
fn test_get_workspace() {
    let mock_store = Arc::new(MockWaveStore::new());
    let service = WorkspaceService::new(mock_store);
    let ws = service.get_workspace("test-id").unwrap();
    assert_eq!(ws.name, "Test Workspace");
}
```

### 2. Injection Points for Debugging

With services, we can inject debug hooks:

```rust
pub trait WorkspaceStore {
    fn get_workspace(&self, id: &str) -> Result<Workspace>;
}

pub struct DebugWorkspaceStore {
    inner: Arc<dyn WorkspaceStore>,
}

impl WorkspaceStore for DebugWorkspaceStore {
    fn get_workspace(&self, id: &str) -> Result<Workspace> {
        println!("[DEBUG] Getting workspace: {}", id);
        let result = self.inner.get_workspace(id);
        println!("[DEBUG] Result: {:?}", result);
        result
    }
}
```

### 3. Contract Testing

**RPC Contract**:
```rust
// Ensure frontend and backend agree on types
#[test]
fn test_workspace_json_schema() {
    let ws = Workspace::default();
    let json = serde_json::to_value(&ws).unwrap();

    assert!(json.get("oid").is_some());
    assert!(json.get("otype").is_some());
    assert_eq!(json["otype"], "workspace");
}
```

---

## Root Cause Hypothesis: The Smoking Gun

After this analysis, I believe the issue is:

### **The CSS is applied but immediately overridden by Tailwind's reset**

**Evidence**:
1. Bundled CSS has body styles ✅
2. CSS loads before JS ✅
3. Variables are defined ✅
4. But Tailwind CSS v4 is also bundled

**Tailwind Reset** (in `index-{hash}.css`):
```css
/* Tailwind's preflight (reset) */
*, ::before, ::after {
    /* ... */
}
```

Tailwind's reset might be resetting the body background AFTER our styles.

**Test**: Check CSS specificity and order in bundled file
```bash
grep -n "body{" dist/frontend/assets/index-*.css
```

The output showed **three** body rules:
1. Line X: `body{line-height:1.2;-webkit-font-smoothing:antialiased}`
2. Line Y: `body{line-height:1.2;-webkit-font-smoothing:antialiased}` (duplicate)
3. Line Z: `body{display:flex;...;background-color:var(--main-bg-color);...}`

If rules 1 and 2 come AFTER rule 3, they override without setting background-color, reverting to default.

### **Solution**: Increase Specificity or Force Order

**Option A**: Add specificity
```scss
// app.scss
body.init,
body.init.is-transparent {
    background-color: var(--main-bg-color);
}

body.init.is-transparent {
    background-color: transparent;
}
```

**Option B**: Use `!important` (temporary)
```scss
body {
    background-color: var(--main-bg-color) !important;
}
```

**Option C**: Configure Tailwind to not reset body
```js
// tailwind.config.js
module.exports = {
    corePlugins: {
        preflight: false,  // Disable reset
    },
}
```

---

## Recommended Immediate Actions

### 1. Verify CSS Order
```bash
cd /c/Systems/wavemux/dist/frontend/assets
grep -n "body{" index-*.css | cat -n
```

Look for:
- Which body rule is LAST?
- Does last rule set background-color?

### 2. Test Specificity Fix

Edit `frontend/app/app.scss`:
```scss
body,
body.init {
    display: flex;
    flex-direction: row;
    width: 100vw;
    height: 100vh;
    color: var(--main-text-color);
    font: var(--base-font);
    overflow: hidden;
    background-color: var(--main-bg-color) !important;  // Force it
    -webkit-font-smoothing: auto;
    backface-visibility: hidden;
    transform: translateZ(0);
}
```

Rebuild and test.

### 3. Add Visual Debugging

The current build (0.20.19) has enhanced logging. Run it and call:
```javascript
window.dumpState()
```

This will show:
- Computed body background
- Body dimensions
- Whether styles are applied

### 4. Enable Debug Mode

In the console:
```javascript
window.enableDebugMode()
```

If you see:
- **RED body**: CSS IS working, just wrong color variable
- **No change**: CSS NOT being applied at all

---

## Long-Term: Modularization Benefits for Bug Prevention

By implementing the modular architecture:

1. **Rendering Module**: Would have self-tests that catch this issue
   ```typescript
   // rendering.test.ts
   expect(getBackgroundColor()).not.toBe('grey');
   ```

2. **Theme Service**: Would validate theme application
   ```typescript
   applyTheme(darkTheme);
   if (!verifyThemeApplied(darkTheme)) {
       throw new Error('Theme not applied!');
   }
   ```

3. **Contract Tests**: Would ensure CSS variables are set
   ```typescript
   expect(getCSSVariable('--main-bg-color')).toBe('rgb(34, 34, 34)');
   ```

4. **Visual Regression Tests**: Would catch UI changes
   - Screenshot on each build
   - Compare with baseline
   - Alert on differences

---

## Conclusion

The grey screen bug is likely a **CSS specificity/order issue** where Tailwind's reset or a duplicate body rule is overriding our background color. The modular architecture would have made this easier to debug and prevent in the first place.

**Next Steps**:
1. Run 0.20.19 with enhanced debug logging
2. Call `window.dumpState()` to see computed values
3. Check CSS bundle order with grep
4. Apply specificity fix with `!important`
5. Begin Phase 1 of modularization (extract domain models)

**Files to Create**:
- [ ] `src-tauri/src/domain/` directory structure
- [ ] `src-tauri/src/services/` directory structure
- [ ] `frontend/app/services/` directory structure
- [ ] `frontend/app/models/` directory structure
- [ ] Unit tests for each module

This architecture will make AgentMux more maintainable, testable, and debuggable going forward.

---

*End of Analysis*
