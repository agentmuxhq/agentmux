# Case Sensitivity Fix: Agent ID Normalization

**Author:** AgentA
**Date:** 2026-01-17
**Status:** Draft

## Problem Statement

AgentX had `WAVEMUX_AGENT_ID="AgentX"` but injections sent to `"agentx"` weren't delivered.

**Root cause:** Case mismatch between:
- WaveMux poller polling for `"AgentX"` (as registered locally)
- AgentMux cloud storing injections under `"agentx"` (normalized)

## Current State

### AgentMux Cloud (Already Fixed)
- All agent IDs normalized to lowercase on storage
- Query for `"AgentX"`, `"AGENTX"`, `"agentx"` all return same results
- ✅ Case insensitive

### WaveMux Poller (Bug)
- Polls using exact agent ID from local registration
- If registered as `"AgentX"`, polls `/reactive/pending/AgentX`
- AgentMux returns injections for `"agentx"` but URL mismatch may cause issues
- ❌ Not normalized

### Shell Integration
- Sends whatever is in `WAVEMUX_AGENT_ID` env var
- No normalization
- ❌ Case preserved

## Design Principles

1. **Normalize early, normalize everywhere** - Convert to lowercase at system boundaries
2. **Backward compatible** - Existing configs with any case should work
3. **Consistent** - Same agent ID format across all components

## Solution

### Where to Normalize

| Component | Location | Action |
|-----------|----------|--------|
| WaveMux Handler | `RegisterAgent()` | Normalize agentID to lowercase |
| WaveMux Poller | `pollForAgent()` | Normalize agentID to lowercase |
| WaveMux Poller | `acknowledgeDelivery()` | Normalize agentID to lowercase |
| Shell Integration | OSC handler | Normalize before registration |

### Implementation

#### 1. handler.go - Normalize on Registration

```go
// RegisterAgent associates an agent ID with a block ID.
func (h *Handler) RegisterAgent(agentID, blockID, tabID string) error {
    // Normalize agent ID to lowercase for case-insensitive matching
    agentID = strings.ToLower(agentID)

    h.mu.Lock()
    defer h.mu.Unlock()
    // ... rest of function
}
```

#### 2. poller.go - Normalize on Poll

```go
// pollForAgent polls for pending injections for a specific agent.
func (p *Poller) pollForAgent(agentID string) error {
    // Normalize agent ID to lowercase (AgentMux stores all IDs lowercase)
    agentID = strings.ToLower(agentID)

    // Build request URL
    reqURL := fmt.Sprintf("%s/reactive/pending/%s", p.agentmuxURL, url.PathEscape(agentID))
    // ... rest of function
}
```

#### 3. poller.go - Add strings import

```go
import (
    "bytes"
    "context"
    "encoding/json"
    "fmt"
    "io"
    "log"
    "net/http"
    "net/url"
    "os"
    "path/filepath"
    "strings"  // ADD THIS
    "sync"
    "time"

    "github.com/a5af/wavemux/pkg/wavebase"
)
```

### Files to Modify

1. `pkg/reactive/handler.go` - Normalize in `RegisterAgent()`
2. `pkg/reactive/poller.go` - Add `strings` import, normalize in `pollForAgent()` and `acknowledgeDelivery()`

### Testing

1. Set `WAVEMUX_AGENT_ID="AgentX"` (mixed case)
2. Start WaveMux, verify registration logs show `"agentx"`
3. Send injection to `"agentx"` from another agent
4. Verify injection delivered within 5 seconds

### Documentation Updates

Update these files to note case insensitivity:
- `docs/specs/CROSS_HOST_REACTIVE_MESSAGING.md`
- `CLAUDE.md` (if applicable)

## Alternatives Considered

### Option A: Normalize only in poller (Chosen - Minimal Change)
- Normalize agentID in `pollForAgent()` before making request
- Quick fix, minimal code change
- Cons: Registration still stores original case

### Option B: Normalize at registration (Better Long-term)
- Normalize in `RegisterAgent()`
- All internal storage is lowercase
- Pros: Consistent everywhere
- Cons: More changes needed

### Option C: Normalize everywhere (Most Robust)
- Normalize at all boundaries
- Registration, polling, injection, acknowledgment
- Pros: Bulletproof
- Cons: More code changes

**Recommendation:** Start with Option A for immediate fix, then apply Option C for robustness.

## Timeline

1. Implement Option A (poller normalization) - immediate
2. Test cross-host injection
3. If working, create PR
4. Consider Option C in follow-up PR
