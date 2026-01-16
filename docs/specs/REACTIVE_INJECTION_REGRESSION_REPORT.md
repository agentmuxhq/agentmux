# Reactive Injection Regression Report

**Date:** 2026-01-16
**Author:** AgentA
**Status:** In Progress
**Severity:** High - Core functionality broken

---

## Executive Summary

Cross-host reactive message injection has regressed. Messages appear in the terminal but are not submitted (Enter key not processed). Multiple fix attempts have made the situation worse or introduced new issues.

---

## Timeline

### v0.15.x (Working Baseline)
- **Original implementation** worked reliably
- Synchronous message + Enter sending
- 100ms delay between message and Enter
- Single `\r` for Enter key

### v0.16.0 - v0.16.1 (2026-01-15)
- Added cross-host reactive messaging poller (PR #144)
- Poller polls AgentMux Lambda every 5 seconds
- Local injection still worked

### v0.16.2 (2026-01-16 ~01:10)
- **Commit ee2cf08**: Made Enter key async to "prevent DoS"
- Changed from synchronous to goroutine-based Enter sending
- **This was the breaking change**

### v0.16.3 (2026-01-16 ~04:47)
- Attempted fix: Increased delay from 100ms to 300ms
- Changed `\r` to `\r\n`
- Added retry after 700ms
- **Result:** Still broken - Enter keys created blank lines but didn't submit message

### v0.16.4 (2026-01-16 ~05:08)
- Added third retry attempt (300ms → 500ms → 500ms)
- Added documentation explaining the design
- **Result:** 4 blank lines created, message still not submitted

### v0.16.5 (Not released)
- PR #146 created: Revert to synchronous + add rate limiter
- Not yet tested

---

## Root Cause Analysis

### The Breaking Change

```go
// BEFORE (worked):
err := h.inputSender(blockID, []byte(finalMsg))
time.Sleep(100 * time.Millisecond)
err = h.inputSender(blockID, []byte("\r"))  // synchronous
return response

// AFTER (broken):
err := h.inputSender(blockID, []byte(finalMsg))
go func() {  // async goroutine
    time.Sleep(300 * time.Millisecond)
    h.inputSender(blockID, []byte("\r\n"))
}()
return response  // returns before Enter is sent
```

### Why Async Breaks It

1. **Message and Enter become disconnected** - The HTTP response returns before Enter is sent
2. **Terminal state may change** - By the time the goroutine runs, the terminal context may have shifted
3. **PTY buffer coordination lost** - The message and Enter need to be part of the same "transaction"

### Why Retries Made It Worse

- Multiple Enter keys all fire successfully
- But they fire on empty input (message already "gone" from input context)
- Result: blank lines instead of submitted message

---

## What We Know Works

1. **Synchronous sending** - Message + delay + Enter in sequence, blocking
2. **Single `\r`** - Not `\r\n` (may cause double line endings)
3. **~100-150ms delay** - Enough for terminal to process, not too long

## What We Know Doesn't Work

1. **Async Enter sending** - Goroutine breaks message/Enter coordination
2. **Multiple retries** - Creates blank lines
3. **`\r\n` instead of `\r`** - May contribute to issues
4. **Long delays (300ms+)** - Doesn't help, may hurt

---

## Proposed Solution

### Option A: Simple Revert (Recommended)

Revert to the original synchronous approach:

```go
// Send message
err := h.inputSender(blockID, []byte(finalMsg))
if err != nil {
    return errorResponse(err)
}

// Small delay
time.Sleep(150 * time.Millisecond)

// Send Enter (synchronous, single \r)
err = h.inputSender(blockID, []byte("\r"))
if err != nil {
    log.Printf("Enter key failed: %v", err)
}

return successResponse()
```

**DoS Mitigation:** Add rate limiter (10 req/sec) instead of async.

### Option B: Investigate PTY Layer

If Option A doesn't work, investigate:
- How `inputSender` → `blockcontroller.SendInput` → PTY actually works
- Whether there's a higher-level "submit input" API
- Whether WebSocket/RPC path differs from PTY injection

### Option C: Alternative Input Method

Bypass PTY entirely:
- Use the same RPC path as real user input
- `ControllerInputCommand` already exists
- May require frontend changes

---

## Test Plan

1. Build with synchronous Enter (PR #146 or new commit)
2. Deploy to area54 and gamerlove
3. Test single injection - verify message is submitted
4. Test 10-round improv game
5. Verify no blank lines created
6. Verify rate limiter works (>10 req/sec should fail)

---

## Files Modified

| File | Changes |
|------|---------|
| `pkg/reactive/handler.go` | Enter key timing and async logic |
| `pkg/reactive/poller.go` | Cross-host polling (not related to bug) |

---

## PRs

| PR | Status | Description |
|----|--------|-------------|
| #144 | Merged | Cross-host reactive messaging poller |
| #145 | Merged | Enter key timing fix (made it worse) |
| #146 | Open | Revert to sync + rate limiter |

---

## Immediate Next Steps

1. **Test PR #146** - Build and verify synchronous approach works
2. **If works:** Merge PR #146, release v0.16.5
3. **If doesn't work:** Investigate PTY layer more deeply

---

## Lessons Learned

1. **Don't optimize prematurely** - The async change was to prevent theoretical DoS, but broke real functionality
2. **Test after each change** - The regression wasn't caught because testing was deferred
3. **Simple is better** - The original synchronous approach was correct
4. **Rate limiting > async** - For DoS protection, rate limiting is cleaner than making code async

---

## Appendix: Key Code Locations

- **Handler:** `pkg/reactive/handler.go:202-265` (InjectMessage)
- **Poller:** `pkg/reactive/poller.go` (cross-host polling)
- **Input sender:** `cmd/server/main-server.go` (initReactiveHandler)
- **Block controller:** `pkg/blockcontroller/blockcontroller.go:256` (SendInput)
