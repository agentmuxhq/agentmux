# Retro: Rust Backend Session 2 - Debug Loop

**Date:** 2026-02-17

## What Was Accomplished

### Bugs Fixed
1. **`CreateWindow` with empty workspace_id** — `wcore::create_window` crashed with "not found" when called with `""`. Fixed to auto-create a workspace+tab when `workspace_id` is empty.
2. **Debug logging added** to `wave.ts` `initWave` at granular points between lines 403-429 (before/after `initGlobal`, before/after `initWshrpc`, before `loadConnStatus`).

### Binary rebuilt
- `cargo build --release -p agentmuxsrv-rs` succeeded
- Copied to all 4 locations: `dist/bin/`, `src-tauri/binaries/`, `src-tauri/target/debug/`, `src-tauri/target/release/`

---

## What Went Wrong — The Loop

### Root Cause: Uncontrolled Background Process Spawning

Got into a loop of:
1. Start `task dev` in background
2. Previous Vite dev server still on port 5173 → exit code 201
3. Try again → same result
4. Repeat

Every iteration failed because I didn't cleanly kill the previous Vite/node process before retrying.

### Why This Happened
- Used `run_in_background=true` Bash for `task dev` — output isn't captured the same way as a Task agent
- Used a subagent (Task tool) which spawned its OWN background `task dev`, creating 2 competing instances
- Lost track of which processes were "mine" — multiple agentmux.exe PIDs, multiple node.exe PIDs
- Never confirmed a clean slate before each restart attempt

### Protocol Failures
1. **Didn't kill Vite (node) before restarting** — only killed agentmux.exe and agentmuxsrv-rs.exe, not the Vite node process (PID 33044)
2. **Spawned a subagent to start task dev** — subagent started ANOTHER background process, compounding the problem
3. **Didn't check port 5173 before each attempt**
4. **Kept trying the same failing approach** instead of stopping after the first failure

---

## Remaining Issues (Unresolved)

### Critical: `initWave` debug logs don't appear after "Init Wave"
- Line 403 `getApi().sendLog("Init Wave ...")` appears in logs ✅
- Lines 429+ `getApi().sendLog("[initWave] calling loadConnStatus")` etc. do NOT appear
- This means something between lines 415-428 is failing silently OR the new code wasn't hot-reloaded
- The app DID render (user saw it, opened a new window) — so `finally` block ran
- "Widgets are missing" = tab is empty (no blocks)

### Likely Root Cause of Empty Widgets
- `ensure_initial_data` creates client/workspace/window/tab but NO blocks
- Original WaveTerm created a default shell block on first launch
- Need to create a default block (e.g., `term` widget) in `ensure_initial_data`

### The sendLog silence mystery
- One hypothesis: `initWshrpc` opens a WebSocket which fires events that somehow cause the log pipeline to fail
- Better hypothesis: the code DID run and logs were emitted but the background task file was full/cut off at line 7427 (the log file ended exactly at "Init Wave" which was line 7427 — the file may have been truncated)
- **Check this first on next session**: read the full log file from a fresh run

---

## Correct Restart Protocol

```
1. taskkill /F /IM agentmux.exe /T
2. taskkill /F /IM agentmuxsrv-rs.exe /T
3. tasklist | grep node → kill the Vite process (highest memory node.exe)
4. netstat -ano | grep 5173 → confirm port free
5. del C:\Users\area54\AppData\Roaming\com.a5af.agentmux\instances\default\wave-endpoints.json
6. task dev (in a terminal the user controls, not background)
```

---

## Next Steps

1. User restarts `task dev` manually in their terminal
2. Observe logs — check if "[initWave] calling initGlobal" appears (new debug line)
3. If "widgets are missing" persists: add default block creation to `ensure_initial_data` in `wcore.rs`
4. Fix the stash: apply `stash@{0}` with agent widget POC changes
