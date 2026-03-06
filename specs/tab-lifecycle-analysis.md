# Tab Lifecycle & CreateBlock Failure — Complete Analysis

## Problem

After opening/closing tabs and panes repeatedly, `CreateBlock` calls fail with `"not found"`. The app becomes unable to create new panes until restarted.

```
[UNHANDLED-REJECTION] call object.CreateBlock error: not found
```

## Root Cause

When the active tab is closed, the frontend doesn't switch to another tab before deleting. The `activeTabId` atom continues pointing at the now-deleted tab. Any subsequent operation that uses `activeTabId` (creating panes, splitting, etc.) sends a stale tab ID to the backend, which returns "not found".

### Error Path

```
1. User closes active tab (Tab A)
2. handleClose() calls WorkspaceService.CloseTab(wsId, tabA)
3. Backend deletes Tab A from store
4. Frontend receives WaveObjUpdate with delete for Tab A
5. But activeTabId STILL = Tab A (never switched)
6. User tries to create a pane in the "current tab"
7. ObjectService.CreateBlock sends tabId = Tab A
8. Backend: store.must_get::<Tab>(tabA) → not found
9. Error returned to frontend
```

### Fix Applied (v0.31.58)

In `tabbar.tsx` `handleClose()`: if the tab being closed is the active tab, switch to an adjacent tab first (next, or previous if closing the last tab).

```typescript
if (tabId === activeTabId) {
    const idx = allTabs.indexOf(tabId);
    const nextTab = allTabs[idx + 1] ?? allTabs[idx - 1];
    if (nextTab) setActiveTab(nextTab);
}
```

## Remaining Risks & Deeper Analysis

### 1. Race Condition: setActiveTab vs CloseTab

The fix switches the active tab synchronously, then fires `CloseTab` asynchronously. There's a window where:
- `setActiveTab(nextTab)` triggers a backend call (`SetActiveTab`)
- `CloseTab(tabA)` also goes to the backend
- If `CloseTab` completes first and the workspace update reaches the frontend before `SetActiveTab` completes, the derived `activeTabIdAtom` could momentarily resolve to an empty/stale value

**Mitigation:** `setActiveTab` is called before `CloseTab` starts. The atom update is synchronous on the frontend side (optimistic). The backend calls are ordered — `SetActiveTab` fires first. In practice this race is unlikely but could be hardened with:
- Awaiting `setActiveTab` before calling `CloseTab`
- Or having `CloseTab` return the new active tab ID from the backend

### 2. Backend Doesn't Set New Active Tab

The backend `CloseTab` handler returns `newactivetabid: String::new()`. It deletes the tab but doesn't update the workspace's `activetabid` field. This means:

- If the frontend crashes after calling `CloseTab` but before `setActiveTab`, the workspace persists with a deleted `activetabid`
- On next launch, the frontend loads a workspace pointing to a nonexistent tab

**Fix needed:** Backend `CloseTab` should:
1. Check if the closed tab is the workspace's active tab
2. If so, update `activetabid` to an adjacent tab
3. Return the new active tab ID in `CloseTabRtnType`

### 3. Tab Deletion Doesn't Clean Up Blocks

When a tab is deleted (`wcore::delete_tab`), its blocks may not be cleaned up. Let me verify:

```
File: agentmuxsrv-rs/src/backend/wcore.rs
```

If `delete_tab` only removes the tab from the workspace's `tabids`/`pinnedtabids` and deletes the Tab object, but doesn't delete the blocks within it, those blocks become orphaned in the store — wasting memory and potentially causing stale references.

### 4. Frontend Layout Model Cleanup

`deleteLayoutModelForTab(tabId)` is called after `CloseTab`, but the layout model might already be in use by the still-rendering `TabContent` component. Since React state updates are batched, the component might try to access the deleted layout model during the same render cycle.

### 5. Rapid Tab Operations

Opening and closing tabs rapidly can cause:
- Multiple `CreateTab` / `CloseTab` calls in flight simultaneously
- WaveObjUpdate events arriving out of order
- The workspace's `tabids` array becoming inconsistent between frontend and backend

## Recommendations

### Immediate (v0.31.58 — applied)
- Switch active tab before closing ✅

### Short-term
1. **Backend CloseTab should manage active tab:**
   ```rust
   // In delete_tab or CloseTab handler:
   if ws.activetabid == tab_id {
       let remaining = [&ws.pinnedtabids[..], &ws.tabids[..]].concat();
       ws.activetabid = remaining.first().cloned().unwrap_or_default();
   }
   ```
   Return `newactivetabid` in the response so the frontend can reconcile.

2. **Backend CloseTab should delete blocks:**
   ```rust
   // Delete all blocks belonging to the tab
   for block_id in &tab.blockids {
       store.delete::<Block>(block_id)?;
   }
   ```

3. **Frontend should await setActiveTab before CloseTab:**
   ```typescript
   if (tabId === activeTabId) {
       await setActiveTab(nextTab); // await the backend round-trip
   }
   await WorkspaceService.CloseTab(workspace.oid, tabId);
   ```

### Medium-term
4. **Add retry/recovery for CreateBlock:**
   If `CreateBlock` fails with "not found", re-read `activeTabId` from the workspace object and retry once.

5. **Validate activeTabId on workspace load:**
   On startup, verify that `workspace.activetabid` exists in the store. If not, reset to the first available tab.

6. **Serialize tab operations:**
   Queue tab create/close/switch operations so only one is in flight at a time. This prevents race conditions from rapid clicking.

## Files Involved

| File | Role |
|------|------|
| `frontend/app/tab/tabbar.tsx` | Tab close handler — **fixed** to switch active tab first |
| `frontend/app/store/global.ts` | `createBlock`, `setActiveTab` — uses `activeTabId` for backend calls |
| `frontend/app/store/services.ts` | `ObjectService.CreateBlock` — sends tab ID from UI context |
| `agentmuxsrv-rs/src/server/service.rs` | `CloseTab` handler — doesn't update active tab |
| `agentmuxsrv-rs/src/backend/wcore.rs` | `create_block` — fails with "not found" when tab doesn't exist |
| `agentmuxsrv-rs/src/backend/wcore.rs` | `delete_tab` — may not clean up blocks |
