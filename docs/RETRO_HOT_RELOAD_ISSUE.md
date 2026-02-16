# Retrospective: Hot Reload Failure & Port Mismatch

**Date:** 2026-02-15
**Issue:** Hot module replacement not working; code changes not appearing in running dev instance
**Duration:** ~2 hours of debugging
**Status:** RESOLVED

---

## Executive Summary

Hot reload failed because **multiple stale Vite dev servers** were running on different ports (5174, 5175, 5176), while Tauri was configured to load from port **5173**. This caused a mismatch where:
1. The Tauri webview loaded from a stale dev server (or cached build)
2. New changes were being served by a different Vite instance
3. The two were never synchronized

**Root Causes:**
1. Vite's default behavior (auto-increment port when in use) masked the problem
2. No enforcement that Tauri's `devUrl` must match Vite's port
3. Multiple zombie Vite processes from previous failed dev sessions
4. Aggressive cleanup (deleting `dist/frontend`) broke Tauri compilation

---

## Timeline

### T+0: User Reports Issue
- **Symptom:** "The bell icon should be gone but appears on dev"
- **Expected:** Notification bell removed in PR #316
- **Actual:** Bell still visible, along with old button layout

### T+10min: Initial Investigation
- Verified code changes were saved to disk ✓
- Verified git diff showed correct modifications ✓
- Assumed browser cache issue ❌

### T+20min: Discovery of Port Mismatch
- **Key finding:** Vite dev server running on port **5175** instead of **5173**
- **Output:**
  ```
  Port 5173 is in use, trying another one...
  Port 5174 is in use, trying another one...
  ➜  Local:   http://localhost:5175/
  ```
- **Tauri config:** Hardcoded to `devUrl: "http://localhost:5173"`
- **Conclusion:** Webview loading from wrong port

### T+30min: Attempted Fix #1 - Kill Port 5173 Process
- Killed PID 23384 (process using 5173)
- Restarted dev server
- **Result:** Still failed ❌
- **Why:** Multiple zombie Vite servers still running on 5174, 5175, 5176

### T+45min: Discovery of Multiple Stale Servers
- **netstat revealed:**
  ```
  TCP    [::1]:5174    LISTENING    26368
  TCP    [::1]:5175    LISTENING    17768
  TCP    [::1]:5176    LISTENING    26888
  ```
- These were from previous failed `task dev` attempts
- Webview likely cached one of these

### T+60min: Attempted Fix #2 - Nuclear Cleanup
- Killed all processes: 26368, 17768, 26888
- Deleted `dist/frontend` to clear compiled cache
- Deleted `node_modules/.vite` to clear Vite cache
- **Result:** Tauri compilation failed ❌
- **Why:** Tauri requires `dist/frontend` to exist (even in dev mode)

### T+75min: Discovery of Tauri Build Requirement
- **Error:** `proc macro panicked: frontendDist "../dist/frontend" doesn't exist`
- **Insight:** Tauri's build macro validates path at compile time
- Even though dev mode uses Vite dev server, the directory must exist

### T+90min: Final Fix
1. Created minimal `dist/frontend/index.html` (dummy file)
2. Killed ALL node and agentmux processes
3. Verified port 5173 was clear
4. Added `strictPort: true` to `vite.config.tauri.ts`
5. Started dev server fresh
6. **Result:** SUCCESS ✓

---

## Root Cause Analysis

### Primary Cause: Vite Port Auto-Increment
When Vite can't bind to the configured port (5173), it automatically tries the next available port. This is helpful for development, but **dangerous** when a framework (Tauri) has a hardcoded `devUrl`.

**Why ports were in use:**
- Failed `task dev` runs don't always clean up Vite processes
- On Windows, zombie Node processes can persist after Ctrl+C
- Each new `task dev` spawns a new Vite server without killing the old one

### Secondary Cause: No Port Validation
There was no mechanism to:
1. Verify Tauri's `devUrl` matches Vite's actual port
2. Fail fast if port 5173 is unavailable
3. Warn when Vite auto-increments to a different port

### Contributing Factor: Aggressive Cache Clearing
Deleting `dist/frontend` to "clear the cache" broke Tauri compilation, creating a red herring that delayed the real fix.

---

## The Fix

### 1. Enforce Strict Port (Immediate)
**File:** `vite.config.tauri.ts`
```diff
  server: {
      port: 5173,
+     strictPort: true, // Fail if port 5173 is already in use (required for Tauri)
      open: false,
```

**Effect:** Vite will now **error** instead of silently using a different port.

**Output when port in use:**
```
error when starting dev server:
Error: Port 5173 is already in use
```

This makes the problem **visible** instead of silent.

### 2. Process Cleanup (Operational)
Before running `task dev`, ensure no zombie Vite servers:
```bash
# Windows
powershell -Command "Get-Process node | Where-Object {$_.CommandLine -like '*vite*'} | Stop-Process -Force"

# Or simpler (kills ALL node):
powershell -Command "Stop-Process -Name node -Force"
```

### 3. Preserve `dist/frontend` (Build System)
Never delete `dist/frontend` during development. If cache clearing is needed:
```bash
# Clear Vite cache only
rm -rf node_modules/.vite

# NOT this:
# rm -rf dist/frontend  ❌
```

---

## Preventive Measures

### For Developers
1. **Always check ports before starting dev:**
   ```bash
   netstat -ano | findstr :5173
   ```
2. **Kill zombie processes regularly:**
   ```bash
   task clean-processes  # (new task to add)
   ```
3. **Never delete `dist/frontend` manually**

### For Build System
1. ✅ **Add** `strictPort: true` to Vite config (DONE)
2. **Add** pre-dev hook to Taskfile:
   ```yaml
   dev:
     deps:
       - kill-zombie-processes
     cmds:
       - npx tauri dev
   ```
3. **Add** `clean-processes` task:
   ```yaml
   clean-processes:
     cmds:
       - powershell -Command "Stop-Process -Name node,agentmux,agentmuxsrv -Force -ErrorAction SilentlyContinue"
   ```

### For CI/CD
- Ensure test environments don't accumulate zombie Vite processes
- Add health check: verify Vite is actually on 5173 before running tests

---

## Lessons Learned

### What Went Wrong
1. **Silent failures are dangerous:** Vite auto-incrementing ports masked the real problem for 30+ minutes
2. **Over-aggressive fixes can backfire:** Deleting `dist/frontend` created a new problem
3. **Multiple sources of truth:** Tauri config (5173) vs Vite runtime port (517x) weren't validated

### What Went Right
1. **Systematic debugging:** Eventually traced through netstat to find multiple zombie processes
2. **Adding enforcement:** `strictPort: true` prevents this class of bug in the future
3. **User feedback loop:** "The bell is still there" was the critical clue that we weren't loading the right code

### Best Practices Going Forward
1. **Always validate configuration consistency** (Tauri devUrl === Vite port)
2. **Fail fast, fail loud** (strict mode > silent fallbacks)
3. **Process hygiene matters** (clean up zombies between dev sessions)

---

## Verification Checklist

After `task dev` starts, verify:
- [ ] Vite output shows: `➜  Local:   http://localhost:5173/`
- [ ] No other Vite servers running: `netstat -ano | findstr :517`
- [ ] Tauri compiled without errors
- [ ] Backend shows: `Backend ready: ws=...`
- [ ] Code changes hot-reload within 2 seconds
- [ ] No stale UI elements (notification bell, wrong button positions)

---

## Files Modified

1. `vite.config.tauri.ts` - Added `strictPort: true`
2. `frontend/app/window/window-controls.tsx` - Moved min/max buttons
3. `frontend/app/window/system-status.tsx` - Added WindowActionButtons
4. `frontend/app/window/system-status.scss` - Styled action buttons
5. `frontend/app/window/action-widgets.tsx` - Removed NotificationPopover

---

## Related Issues

- PR #316: Removed notification bell (changes lost during refactor)
- PR #319: Renamed widgetbar.tsx → action-widgets.tsx (merge conflict)
- Window controls refactoring (Phases 1-3)

---

**Author:** AgentA (Claude)
**Reviewed:** User (area54)
**Status:** Fixed ✓
**Prevention:** `strictPort: true` enforced
