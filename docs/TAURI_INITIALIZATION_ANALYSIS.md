# AgentMux Tauri Initialization Analysis

**Date:** 2026-02-08
**Author:** Claude (AgentA)
**Context:** Investigating grey screen issue in Tauri port

---

## Executive Summary

### The Problem

The Tauri port shows a **grey screen** on launch due to incomplete initialization. While authentication works perfectly (no 401 errors), the frontend cannot render because:

1. ✅ **ClientId**: Valid (fetched from backend)
2. ✅ **WindowId**: Valid (fetched from backend)
3. ❌ **TabId**: Empty (window object doesn't exist in database)
4. ❌ **Window/Workspace objects**: Missing from database despite being in client.windowids[]

### Root Cause

**Timing mismatch**: The Tauri implementation fetches client data BEFORE the backend has fully initialized window/workspace/tab objects. The backend's `EnsureInitialData()` only guarantees a Client exists, but actual Window objects may not be created yet.

**Electron doesn't have this problem** because it uses a completely different initialization pattern.

---

## How Electron Initializes (The Working Pattern)

### High-Level Sequence

```
1. Backend starts → EnsureInitialData() creates Client + Window + Workspace + Tab
2. Electron starts → waits for backend ready
3. Electron calls relaunchBrowserWindows()
   ├─ Fetch ClientData (has windowids[])
   ├─ For each windowId:
   │  ├─ Fetch Window object from backend
   │  ├─ Fetch Workspace object from backend
   │  ├─ Create Electron BrowserWindow
   │  └─ Send wave-init with ALL valid IDs
   └─ Show windows
4. Frontend receives wave-init with complete data
5. Frontend loads objects and renders ✅
```

### Key Files and Flow

#### 1. Backend Initialization (`agentmuxsrv` startup)

**File**: `cmd/server/main-server.go:448`

```go
firstLaunch, err := wcore.EnsureInitialData()
```

**File**: `pkg/wcore/wcore.go:25-70`

```go
func EnsureInitialData() (bool, error) {
    // Get or create Client
    client, err := wstore.DBGetSingleton[*waveobj.Client](ctx)
    if err == wstore.ErrNotFound {
        client, err = CreateClient(ctx)  // Creates new client
        firstLaunch = true
    }

    // If client has no windows, CREATE ONE
    if len(client.WindowIds) == 0 {
        if firstLaunch {
            // Create "Starter workspace" with green wave logo
            starterWs, err := CreateWorkspace(ctx, "Starter workspace",
                "custom@wave-logo-solid", "#58C142", false, true)
            wsId = starterWs.OID
        }
        // Create window (which creates workspace + tab if needed)
        _, err = CreateWindow(ctx, nil, wsId)
    }
    return firstLaunch, nil
}
```

**Critical**: This creates the actual Window/Workspace/Tab **objects in the database**, not just IDs.

#### 2. Window Creation Chain

**File**: `pkg/wcore/window.go:81-128`

```go
func CreateWindow(ctx context.Context, winSize *waveobj.WinSize, workspaceId string) (*waveobj.Window, error) {
    var ws *waveobj.Workspace
    if workspaceId == "" {
        // Create new workspace (which creates a tab)
        ws, err = CreateWorkspace(ctx, "", "", "", false, false)
    }

    // Create Window object
    window := &waveobj.Window{
        OID:         uuid.NewString(),
        WorkspaceId: ws.OID,
        IsNew:       true,
        WinSize:     *winSize,  // May be {0, 0}
    }
    err := wstore.DBInsert(ctx, window)  // ← Window written to DB

    // Add to client's window list
    client.WindowIds = append(client.WindowIds, windowId)
    err = wstore.DBUpdate(ctx, client)

    return GetWindow(ctx, windowId)  // Return full object
}
```

#### 3. Workspace + Tab Creation

**File**: `pkg/wcore/workspace.go:51-75`

```go
func CreateWorkspace(ctx, name, icon, color string, applyDefaults, isInitialLaunch bool) (*waveobj.Workspace, error) {
    ws := &waveobj.Workspace{
        OID:          uuid.NewString(),
        TabIds:       []string{},
        PinnedTabIds: []string{},
    }
    err := wstore.DBInsert(ctx, ws)  // ← Workspace written to DB

    // Every workspace gets at least one tab
    _, err = CreateTab(ctx, ws.OID, "", true, false, isInitialLaunch)

    return ws, nil
}
```

**File**: `pkg/wcore/workspace.go:201-231`

```go
func CreateTab(ctx, workspaceId, tabName string, activateTab, pinned, isInitialLaunch bool) (string, error) {
    tab, err := createTabObj(ctx, workspaceId, tabName, pinned || isInitialLaunch, nil)
    // ↑ Tab written to DB

    if activateTab {
        err = SetActiveTab(ctx, workspaceId, tab.OID)
    }

    // Initial launch tabs have NO layout (applied after TOS)
    if !isInitialLaunch {
        err = ApplyPortableLayout(ctx, tab.OID, GetNewTabLayout(), true)
    }

    return tab.OID, nil
}
```

**Result**: Database now contains Client, Window, Workspace, Tab objects.

#### 4. Electron Main Process

**File**: `emain/emain.ts:710-796`

```typescript
async function appMain() {
    // Launch agentmuxsrv backend
    await runWaveSrv(handleWSEvent);
    await getWaveSrvReady();  // ← Wait for backend startup

    // Backend has now run EnsureInitialData()
    // All objects exist in database

    await electronApp.whenReady();
    initElectronWshrpc(ElectronWshClient, { authKey: AuthKey });

    // RELAUNCH WINDOWS (reads from database)
    await relaunchBrowserWindows();

    makeAppMenu();
}
```

#### 5. Relaunch Browser Windows

**File**: `emain/emain-window.ts:807-846`

```typescript
export async function relaunchBrowserWindows() {
    setGlobalIsRelaunching(true);

    // FETCH CLIENT from backend
    const clientData = await ClientService.GetClientData();
    // clientData.windowids = ["uuid-1234", ...]

    const windowIds = clientData.windowids ?? [];

    // FOR EACH window ID, fetch the ACTUAL window object
    for (const windowId of windowIds) {
        const windowData = await WindowService.GetWindow(windowId);
        // ↑ This succeeds because Window was created by EnsureInitialData()

        if (windowData == null) {
            // Window missing → clean up
            await WindowService.CloseWindow(windowId, true);
            continue;
        }

        // Create Electron BrowserWindow
        const win = await createBrowserWindow(windowData, fullConfig, {
            isPrimaryStartupWindow: windowId === primaryWindowId
        });
        wins.push(win);
    }

    // Show all windows
    for (const win of wins) {
        win.show();
    }
}
```

#### 6. Create Browser Window

**File**: `emain/emain-window.ts:656-679`

```typescript
async function createBrowserWindow(waveWindow, fullConfig, opts) {
    // Fetch workspace
    let workspace = await WorkspaceService.GetWorkspace(waveWindow.workspaceid);

    if (!workspace) {
        // RECOVERY: workspace missing → recreate window
        await WindowService.CloseWindow(waveWindow.oid, true);
        waveWindow = await WindowService.CreateWindow(null, "");
        workspace = await WorkspaceService.GetWorkspace(waveWindow.workspaceid);
    }

    // Create Electron window
    const bwin = new WaveBrowserWindow(waveWindow, fullConfig, opts);

    // Set active tab (workspace has activetabid)
    if (workspace.activetabid) {
        await bwin.setActiveTab(workspace.activetabid, false,
            opts.isPrimaryStartupWindow ?? false);
    }
    return bwin;
}
```

#### 7. Send wave-init to Frontend

**File**: `emain/emain-window.ts:368-389`

```typescript
private async initializeTab(tabView: WaveTabView, primaryStartupTab: boolean) {
    const clientId = await getClientId();
    await tabView.initPromise;

    // CREATE complete init options
    const initOpts: WaveInitOpts = {
        tabId: tabView.waveTabId,        // ✅ Valid UUID
        clientId: clientId,               // ✅ Valid UUID
        windowId: this.waveWindowId,      // ✅ Valid UUID
        activate: true,
    };
    if (primaryStartupTab) {
        initOpts.primaryTabStartup = true;
    }

    // SEND to frontend (ALL IDs are valid)
    console.log("sending wave-init", tabView.waveTabId);
    tabView.webContents.send("wave-init", initOpts);

    await tabView.waveReadyPromise;
}
```

#### 8. Frontend Receives wave-init

**File**: `frontend/wave.ts:194-259`

```typescript
async function initWave(initOpts: WaveInitOpts) {
    console.log("Wave Init", "tabid", initOpts.tabId,
        "clientid", initOpts.clientId, "windowid", initOpts.windowId);

    // ALL IDs are guaranteed to be valid
    // Load objects from backend
    const [client, waveWindow, initialTab] = await Promise.all([
        WOS.loadAndPinWaveObject<Client>(WOS.makeORef("client", initOpts.clientId)),
        WOS.loadAndPinWaveObject<WaveWindow>(WOS.makeORef("window", initOpts.windowId)),
        WOS.loadAndPinWaveObject<Tab>(WOS.makeORef("tab", initOpts.tabId)),
    ]);
    // ✅ All succeed - objects exist in database

    const [ws, layoutState] = await Promise.all([
        WOS.loadAndPinWaveObject<Workspace>(WOS.makeORef("workspace", waveWindow.workspaceid)),
        WOS.reloadWaveObject<LayoutState>(WOS.makeORef("layout", initialTab.layoutstate)),
    ]);

    // Render React app
    const reactElem = createElement(App, ...);
    root.render(reactElem);

    // Signal ready
    getApi().setWindowInitStatus("wave-ready");
}
```

**Result**: Frontend renders successfully ✅

---

## How Tauri Currently Tries to Initialize (The Broken Pattern)

### Current Sequence

```
1. Backend starts → EnsureInitialData() creates Client + maybe Window
2. Tauri starts → spawns sidecar
3. Frontend loads → calls set_window_init_status("ready")
4. Rust calls GetClientData → gets client with windowids[]
5. Rust tries to fetch Window object → ERROR: "invalid object reference"
   (Window object doesn't exist yet in DB)
6. Rust emits wave-init with clientId + windowId but EMPTY tabId
7. Frontend receives wave-init
8. Frontend tries to load objects → ERROR: "invalid object reference: ''"
9. Grey screen ❌
```

### Why It Fails

**The problem**: We're calling `GetClientData` immediately, but the backend may return window IDs that don't have corresponding Window objects in the database yet.

**Why this happens**:
1. `EnsureInitialData()` adds window IDs to `client.WindowIds[]`
2. But on some race conditions or incomplete bootstraps, the actual Window objects aren't written to DB
3. OR the Window exists but Workspace/Tab don't

**Evidence from logs**:
```
Got client data: clientId=ee283990-4633-42e0-bd8c-76ebe49957fd,
                 windowId=a1e6a75a-c476-445f-a3cb-3f5b0e5d7249

Window data: {
  "error": "invalid object reference: \"a1e6a75a-c476-445f-a3cb-3f5b0e5d7249\""
}
```

The client has a window ID, but `GetObject(windowId)` fails → object doesn't exist.

### Current Implementation

**File**: `src-tauri/src/commands/stubs.rs:75-145`

```rust
#[tauri::command]
pub async fn set_window_init_status(
    status: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if status == "ready" {
        // Get backend endpoint and auth key
        let (web_endpoint, auth_key) = { ... };

        // Call GetClientData
        let url = format!("http://{}/wave/service?...", web_endpoint, auth_key);
        let rpc_body = serde_json::json!({
            "service": "client",
            "method": "GetClientData",
            "args": [],
        });

        let response = client.post(&url).json(&rpc_body).send().await?;
        let client_data: Value = response.json().await?;

        // Extract IDs
        let data = client_data.get("data").unwrap_or(&client_data);
        let client_id = data.get("oid")...;
        let window_id = data.get("windowids")[0]...;

        // Try to fetch Window object
        let window_rpc = serde_json::json!({
            "service": "object",
            "method": "GetObject",
            "args": [window_id],  // ← FAILS HERE
        });
        let window_response = client.post(&url).json(&window_rpc).send().await?;
        // ERROR: "invalid object reference"

        // Emit wave-init with incomplete data
        window.emit("wave-init", serde_json::json!({
            "clientId": client_id,
            "windowId": window_id,
            "tabId": "",  // ← EMPTY
        }));
    }
    Ok(())
}
```

**Problems**:
1. Assumes Window object exists (it doesn't)
2. No fallback if Window is missing
3. Doesn't create Window if needed
4. Emits wave-init with empty tabId → frontend fails

---

## Key Differences: Electron vs Tauri

| Aspect | Electron | Tauri (Current) |
|--------|----------|-----------------|
| **When objects are created** | Backend startup (`EnsureInitialData`) | Backend startup (same) |
| **When objects are verified** | `relaunchBrowserWindows()` checks each object exists | No verification ❌ |
| **Missing object handling** | Creates new window if missing | Fails with error ❌ |
| **When wave-init is sent** | After loading Window + Workspace + Tab | Immediately (before verification) ❌ |
| **wave-init data completeness** | All 3 IDs always valid ✅ | TabId often empty ❌ |
| **Window size handling** | Electron calculates from screen if 0 | Not handled ❌ |
| **First launch flow** | TOS modal → BootstrapStarterLayout | Not implemented ❌ |

---

## Why Electron's Pattern Works

### 1. **Backend Guarantee**

`EnsureInitialData()` guarantees:
- ✅ Client exists
- ✅ At least one Window exists
- ✅ Each Window has a Workspace
- ✅ Each Workspace has at least one Tab

**All objects are in the database before Electron starts.**

### 2. **Verification Step**

```typescript
// emain-window.ts:820
const windowData = await WindowService.GetWindow(windowId);
if (windowData == null) {
    // Clean up orphaned ID
    await WindowService.CloseWindow(windowId, true);
    continue;
}
```

Electron **verifies each Window object exists** before creating a BrowserWindow.

### 3. **Recovery Mechanism**

```typescript
// emain-window.ts:666
if (!workspace) {
    // Workspace missing → recreate entire window
    await WindowService.CloseWindow(waveWindow.oid, true);
    waveWindow = await WindowService.CreateWindow(null, "");
    workspace = await WorkspaceService.GetWorkspace(waveWindow.workspaceid);
}
```

If any object is missing, **Electron recreates it** before continuing.

### 4. **Complete Data**

```typescript
const initOpts: WaveInitOpts = {
    tabId: tabView.waveTabId,        // ✅ Always valid
    clientId: clientId,               // ✅ Always valid
    windowId: this.waveWindowId,      // ✅ Always valid
    activate: true,
};
```

wave-init is **only sent after all objects are verified to exist**.

---

## Recommended Fixes for Tauri

### Option 1: Mirror Electron's Pattern (Recommended)

**Don't fetch objects in Rust**. Let the **frontend handle initialization** like Electron does.

#### Changes Needed:

1. **Simplify set_window_init_status**

```rust
#[tauri::command]
pub fn set_window_init_status(status: String) {
    // Just store the status, don't emit wave-init
    // Let frontend handle initialization
}
```

2. **Let frontend call GetClientData**

```typescript
// frontend/tauri-init.ts
export async function setupTauriApi(): Promise<void> {
    // Wait for backend ready
    await waitForBackendReady();

    // Fetch client data (like Electron does)
    const clientData = await ClientService.GetClientData();
    const windowId = clientData.windowids?.[0];

    if (!windowId) {
        // No windows → create one
        const newWindow = await WindowService.CreateWindow(null, "");
        windowId = newWindow.oid;
    }

    // Verify window exists
    let windowData = await WindowService.GetWindow(windowId);
    if (!windowData) {
        // Window missing → create new
        windowData = await WindowService.CreateWindow(null, "");
    }

    // Get workspace
    let workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);
    if (!workspace) {
        // Workspace missing → recreate window
        await WindowService.CloseWindow(windowData.oid, true);
        windowData = await WindowService.CreateWindow(null, "");
        workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);
    }

    // Get active tab
    const tabId = workspace.activetabid || workspace.tabids?.[0] || workspace.pinnedtabids?.[0];

    // Now emit wave-init with ALL valid IDs
    const initOpts: WaveInitOpts = {
        clientId: clientData.oid,
        windowId: windowData.oid,
        tabId: tabId,
        activate: true,
        primaryTabStartup: true,
    };

    // Initialize wave
    await initWave(initOpts);
}
```

3. **Remove Rust HTTP calls**

Don't make the Rust layer responsible for backend object fetching. It doesn't have the recovery logic and error handling that the frontend has.

### Option 2: Full Rust Implementation (More Work)

If you want Rust to handle initialization:

1. **Verify objects exist before emitting wave-init**
2. **Implement recovery logic** (create Window if missing)
3. **Implement workspace/tab fetching with fallbacks**
4. **Handle window size = 0 case**
5. **Implement TOS flow**

This is ~500+ lines of Rust code to replicate what the frontend already does.

---

## Immediate Fix (Minimal Changes)

### Short-term: Don't emit wave-init from Rust

**Change**: Remove the backend fetching from `set_window_init_status`, let frontend handle everything.

#### src-tauri/src/commands/stubs.rs

```rust
#[tauri::command]
pub fn set_window_init_status(
    status: String,
    state: tauri::State<'_, AppState>,
) {
    *state.window_init_status.lock().unwrap() = status;
    // Don't emit wave-init - let frontend handle it
}
```

#### frontend/wave.ts

```typescript
async function initBare() {
    // In Tauri, we need to handle initialization ourselves
    if (typeof (window as any).__TAURI_INTERNALS__ !== "undefined") {
        getApi().setWindowInitStatus("ready");

        // Fetch client data and initialize (like Electron's relaunchBrowserWindows)
        await initTauriWave();
    } else {
        // Electron: wait for wave-init event
        getApi().onWaveInit(initWaveWrap);
        getApi().setWindowInitStatus("ready");
    }
}

async function initTauriWave() {
    try {
        // Get client
        const clientData = await ClientService.GetClientData();
        let windowId = clientData.windowids?.[0];

        if (!windowId) {
            const newWindow = await WindowService.CreateWindow(null, "");
            windowId = newWindow.oid;
        }

        // Verify window
        let windowData = await WindowService.GetWindow(windowId);
        if (!windowData) {
            windowData = await WindowService.CreateWindow(null, "");
        }

        // Get workspace
        let workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);
        if (!workspace) {
            await WindowService.CloseWindow(windowData.oid, true);
            windowData = await WindowService.CreateWindow(null, "");
            workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);
        }

        // Get tab
        const tabId = workspace.activetabid ||
                     workspace.tabids?.[0] ||
                     workspace.pinnedtabids?.[0] || "";

        // Initialize with complete data
        const initOpts: WaveInitOpts = {
            clientId: clientData.oid,
            windowId: windowData.oid,
            tabId: tabId,
            activate: true,
            primaryTabStartup: true,
        };

        await initWaveWrap(initOpts);

    } catch (error) {
        console.error("Tauri initialization failed:", error);
        // Show error UI
    }
}
```

**Benefits**:
- Uses existing frontend logic
- Handles missing objects gracefully
- Implements recovery like Electron
- Minimal Rust changes

---

## Testing the Fix

### Before Fix (Current)

```
✅ Backend starts
✅ Auth works
❌ GetClientData returns windowids
❌ GetWindow(windowId) fails
❌ wave-init has empty tabId
❌ Frontend fails to load objects
❌ Grey screen
```

### After Fix

```
✅ Backend starts
✅ Auth works
✅ Frontend calls GetClientData
✅ Frontend verifies Window exists (or creates it)
✅ Frontend verifies Workspace exists (or recreates)
✅ Frontend gets valid Tab ID
✅ wave-init has ALL valid IDs
✅ Frontend loads objects successfully
✅ UI renders
```

---

## References

### Key Files for Implementation

| Component | File | Purpose |
|-----------|------|---------|
| Backend bootstrap | `pkg/wcore/wcore.go` | EnsureInitialData creates objects |
| Window creation | `pkg/wcore/window.go` | CreateWindow logic |
| Workspace creation | `pkg/wcore/workspace.go` | CreateWorkspace + CreateTab |
| Electron startup | `emain/emain.ts` | Main process initialization |
| Electron relaunch | `emain/emain-window.ts` | relaunchBrowserWindows logic |
| Frontend init | `frontend/wave.ts` | initWave implementation |
| Frontend services | `frontend/app/store/services.ts` | RPC proxies |

### Similar Issues in Codebase

**Window size = 0 handling**
- Location: `emain/emain-window.ts:118-138`
- Solution: Calculate from screen dimensions

**Missing workspace handling**
- Location: `emain/emain-window.ts:666-673`
- Solution: Recreate window if workspace missing

**Empty workspace tabs**
- Location: `pkg/wcore/window.go:184-188`
- Solution: Auto-create tab if workspace has none

---

## Conclusion

The **grey screen** is caused by **incomplete initialization** in the Tauri port. The current implementation tries to fetch client/window/tab data from Rust, but:

1. Window objects may not exist in the database
2. No verification or recovery logic
3. wave-init emitted with incomplete data

**Recommended solution**: Move initialization logic to the frontend (TypeScript), where it can:
- Verify objects exist
- Create missing objects
- Handle errors gracefully
- Match Electron's proven pattern

This requires **minimal changes** to Rust (just remove the backend fetching) and **reuses existing frontend logic** that already works in Electron.

---

**Next Steps**:
1. Remove backend object fetching from `set_window_init_status`
2. Implement `initTauriWave()` in frontend
3. Test with fresh database
4. Verify recovery with corrupted data
5. Test window sizing edge cases
