# Spec: Go/Rust Backend Parity Tests

## Problem

The Rust backend (agentmuxsrv-rs) starts correctly and emits `WAVESRV-ESTART`, but the frontend shows a blank screen with `TypeError: Attempted to assign to readonly property`. The same frontend works fine with the Go backend. There are no tests that compare the two backends' responses, so we can't pinpoint the divergence.

## Goal

A test harness that starts both backends, sends identical requests, and diffs the responses. Any field mismatch, missing key, or type difference is a potential cause of the blank screen.

## Architecture

```
scripts/parity-test.sh
  ├── starts Go backend  → port A
  ├── starts Rust backend → port B
  ├── sends same RPC calls to both
  ├── diffs JSON responses (field-by-field)
  └── reports mismatches
```

Single bash script. No new dependencies. Uses `curl` and `jq`.

## RPC Calls to Test

These are the calls made during frontend initialization (`initTauriWave` in `wave.ts`), in order:

| # | Service | Method | Args | Used By |
|---|---------|--------|------|---------|
| 1 | client | GetClientData | none | `wave.ts:114` — gets windowids, oid |
| 2 | window | GetWindow | windowId | `wave.ts:125` — gets workspaceid |
| 3 | window | CreateWindow | null, "" | `wave.ts:120,128,139` — fallback if no window |
| 4 | workspace | GetWorkspace | workspaceId | `wave.ts:134` — gets activetabid, tabids |
| 5 | object | GetObject | oref (tab) | `wos.ts` — loads tab data |
| 6 | object | GetObject | oref (layout) | `wos.ts` — loads layout state |
| 7 | object | GetObjects | oref[] | `wos.ts` — bulk object fetch |
| 8 | filestore | ReadFile | path | config/settings loading |
| 9 | workspace | ListWorkspaces | none | sidebar |
| 10 | object | UpdateObject | oref, data, returnUpdates | mutation path |

## Test Protocol

For each call:

1. Send identical JSON body to both backends: `{"service":"X","method":"Y","args":[...]}`
2. Capture response JSON from both
3. Compare:
   - **Structure**: same keys present in both (recursively)
   - **Types**: string vs number vs bool vs null vs array vs object
   - **Serialization**: field naming (camelCase vs snake_case), empty arrays vs null vs missing
   - **Values**: OIDs will differ, but types and shapes must match
4. Report: `PASS` if shapes match, `FAIL` with diff if not

## What to Ignore

- OID values (UUIDs) — will differ between backends
- Timestamps — will differ
- Version strings — Go reports from build flags, Rust from Cargo.toml
- Port numbers

## What Must Match Exactly

- JSON field names (case-sensitive)
- Field presence (null vs missing vs empty)
- Array vs non-array for list fields
- Object nesting structure
- Boolean values for flags (isnew, hasoldhistory, etc.)

## Response Format Comparison

Go backend response format (from `pkg/web/webcmd/webcmd.go`):
```json
{"success": true, "data": {...}}
{"success": false, "error": "message"}
```

Rust backend response format (from `service.rs:WebReturnType`):
```json
{"success": true, "data": {...}}
{"success": false, "error": "message"}
```

These should match, but verify: field ordering, null handling, empty object/array representation.

## Implementation

### Phase 1: Shape comparison (finds the bug)

```bash
#!/bin/bash
# scripts/parity-test.sh

# 1. Create isolated data dirs for each backend
# 2. Start Go backend, wait for WAVESRV-ESTART, extract port
# 3. Start Rust backend, wait for WAVESRV-ESTART, extract port
# 4. For each RPC call:
#    a. curl Go backend → go_response.json
#    b. curl Rust backend → rs_response.json
#    c. jq 'keys_unsorted' + type checking
#    d. diff and report
# 5. Kill both backends
# 6. Exit 0 if all pass, 1 if any fail
```

### Phase 2: Live initialization replay (optional)

Record the actual WebSocket messages from a working Go backend session, replay them against the Rust backend, compare responses.

## Expected Outcome

The test will identify which RPC response from the Rust backend differs from Go — either a missing field, wrong type, or structural mismatch. That's the cause of `TypeError: Attempted to assign to readonly property`.

## Files

- **Create:** `scripts/parity-test.sh`
- **Read-only reference:** `agentmuxsrv-rs/src/server.rs`, `agentmuxsrv-rs/src/backend/service.rs`, `pkg/web/webcmd/webcmd.go`, `frontend/wave.ts`
