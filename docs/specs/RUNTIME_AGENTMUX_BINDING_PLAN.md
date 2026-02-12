# Runtime AgentMux Environment Variable Binding

**Status:** Implementation Plan
**Author:** AgentA
**Date:** 2026-01-16
**Priority:** High - Required for production deployment

---

## Problem Statement

Currently, `AGENTMUX_URL` and `AGENTMUX_TOKEN` must be set as environment variables **before** AgentMux starts. The poller reads these values once at initialization:

```go
// pkg/reactive/poller.go:316-320
config := PollerConfig{
    AgentMuxURL:   os.Getenv("AGENTMUX_URL"),
    AgentMuxToken: os.Getenv("AGENTMUX_TOKEN"),
}
```

In production, restarting AgentMux to change these values is disruptive. We need runtime reconfiguration like `WAVEMUX_AGENT_ID` already has.

---

## How WAVEMUX_AGENT_ID Works (Reference Implementation)

The existing pattern for `WAVEMUX_AGENT_ID`:

### 1. Shell Integration (OSC 16162)

When a shell sets `WAVEMUX_AGENT_ID`, the wsh shell integration sends an OSC escape sequence:

```bash
# Shell integration sends this to PTY
printf '\033]16162;E;WAVEMUX_AGENT_ID=%s\007' "$agent_id"
```

### 2. Backend Processing

The Go backend receives this via the PTY output parser and calls:

```go
// pkg/blockcontroller/blockcontroller.go
func (bc *BlockController) HandleOscEscape(code int, payload string) {
    if code == 16162 && strings.HasPrefix(payload, "E;") {
        // Parse WAVEMUX_AGENT_ID=value
        // Call reactive.GetGlobalHandler().RegisterAgent(...)
    }
}
```

### 3. Handler Registration

```go
// pkg/reactive/handler.go:94
func (h *Handler) RegisterAgent(agentID, blockID, tabID string) error
```

---

## Implementation Plan

### Option A: Extend OSC 16162 (Recommended)

Extend the existing OSC 16162 mechanism to handle agentmux config.

#### Step 1: Define New OSC Subcommands

```go
// OSC 16162 subcommands:
// E;VAR=value     - Environment variable (existing)
// A;url;token     - AgentMux config (new)
// A;clear         - Clear AgentMux config (new)
```

#### Step 2: Update Shell Integration

Add to `cmd/wsh/cmd/shellintegration.go`:

```go
// New command: wsh agentmux-config <url> <token>
func setAgentMuxConfig(url, token string) {
    // Send OSC 16162;A;url;token to PTY
    fmt.Printf("\033]16162;A;%s;%s\007", url, base64.StdEncoding.EncodeToString([]byte(token)))
}
```

#### Step 3: Update Backend OSC Handler

In `pkg/blockcontroller/blockcontroller.go`:

```go
func (bc *BlockController) HandleOscEscape(code int, payload string) {
    if code != 16162 {
        return
    }

    if strings.HasPrefix(payload, "E;") {
        // Existing: environment variable handling
        bc.handleEnvUpdate(payload[2:])
    } else if strings.HasPrefix(payload, "A;") {
        // New: agentmux config
        bc.handleAgentMuxConfig(payload[2:])
    }
}

func (bc *BlockController) handleAgentMuxConfig(payload string) {
    if payload == "clear" {
        reactive.ReconfigureGlobalPoller("", "")
        return
    }

    parts := strings.SplitN(payload, ";", 2)
    if len(parts) != 2 {
        log.Printf("[blockcontroller] invalid agentmux config: %s", payload)
        return
    }

    url := parts[0]
    tokenBytes, err := base64.StdEncoding.DecodeString(parts[1])
    if err != nil {
        log.Printf("[blockcontroller] invalid agentmux token encoding: %v", err)
        return
    }

    if err := reactive.ReconfigureGlobalPoller(url, string(tokenBytes)); err != nil {
        log.Printf("[blockcontroller] failed to reconfigure poller: %v", err)
    }
}
```

#### Step 4: Implement ReconfigureGlobalPoller

Add to `pkg/reactive/poller.go`:

```go
// ReconfigureGlobalPoller updates the global poller configuration at runtime.
// If url is empty, polling is stopped. If url is set, polling is started/restarted.
func ReconfigureGlobalPoller(agentmuxURL, agentmuxToken string) error {
    globalPollerMu.Lock()
    defer globalPollerMu.Unlock()

    poller := GetGlobalPoller()

    // Stop existing poller if running
    if poller.ctx != nil {
        poller.cancel()
        poller.wg.Wait()
        poller.ctx = nil
        poller.cancel = nil
    }

    // Update configuration
    poller.mu.Lock()
    poller.agentmuxURL = agentmuxURL
    poller.agentmuxToken = agentmuxToken
    poller.mu.Unlock()

    // If URL is empty, leave poller stopped
    if agentmuxURL == "" {
        log.Printf("[reactive/poller] cross-host polling disabled (URL cleared)")
        return nil
    }

    // If token is empty, don't start (require both)
    if agentmuxToken == "" {
        log.Printf("[reactive/poller] cross-host polling disabled (no token)")
        return nil
    }

    // Start the poller with new config
    return poller.Start()
}
```

#### Step 5: Add wsh Command

Create `cmd/wsh/cmd/agentmuxconfig.go`:

```go
package cmd

import (
    "fmt"
    "github.com/spf13/cobra"
)

var agentmuxConfigCmd = &cobra.Command{
    Use:   "agentmux-config [url] [token]",
    Short: "Configure AgentMux connection at runtime",
    Long:  `Sets the AgentMux URL and token for cross-host reactive messaging.

Use 'wsh agentmux-config clear' to disable cross-host polling.`,
    Args: cobra.RangeArgs(1, 2),
    RunE: func(cmd *cobra.Command, args []string) error {
        if args[0] == "clear" {
            // Send clear command
            fmt.Printf("\033]16162;A;clear\007")
            return nil
        }

        if len(args) != 2 {
            return fmt.Errorf("usage: wsh agentmux-config <url> <token>")
        }

        url := args[0]
        token := args[1]

        // Base64 encode token to avoid delimiter issues
        encoded := base64.StdEncoding.EncodeToString([]byte(token))
        fmt.Printf("\033]16162;A;%s;%s\007", url, encoded)

        return nil
    },
}

func init() {
    rootCmd.AddCommand(agentmuxConfigCmd)
}
```

---

### Option B: HTTP Endpoint

Add an HTTP endpoint for runtime configuration (simpler but less secure).

#### Endpoint

```
POST /reactive/config
{
    "agentmux_url": "https://agentmux.asaf.cc",
    "agentmux_token": "bearer-token-here"
}
```

#### Pros
- Simpler implementation
- Can be called from any tool (curl, scripts)

#### Cons
- Security concern: token in HTTP body
- Requires server to be running
- Less integrated with shell workflow

---

### Option C: File Watch

Watch a config file for changes.

#### File Location
```
~/.waveterm/agentmux.json
{
    "url": "https://agentmux.asaf.cc",
    "token": "bearer-token"
}
```

#### Pros
- Easy to script
- Works even before shell integration loads

#### Cons
- Token stored in plaintext file
- Polling/inotify overhead
- Less immediate than OSC approach

---

## Recommended Implementation: Option A

**Rationale:**
1. Follows existing pattern (WAVEMUX_AGENT_ID uses OSC 16162)
2. Secure: token only in memory, not logged
3. Integrates naturally with shell workflow
4. Works from any terminal context via `wsh` command

---

## Implementation Steps

### Phase 1: Core Infrastructure (2 files)

1. **`pkg/reactive/poller.go`**
   - Add `ReconfigureGlobalPoller()` function
   - Add mutex protection for config updates
   - Handle stop/restart logic

2. **`pkg/blockcontroller/blockcontroller.go`**
   - Extend OSC 16162 handler for "A;" prefix
   - Add `handleAgentMuxConfig()` method

### Phase 2: CLI Integration (1 file)

3. **`cmd/wsh/cmd/agentmuxconfig.go`**
   - New `wsh agentmux-config` command
   - Base64 encoding for token safety

### Phase 3: Documentation (1 file)

4. **`docs/agentmux-config.md`**
   - Usage documentation
   - Security considerations
   - Examples

---

## Testing Plan

### Unit Tests

```go
func TestReconfigureGlobalPoller(t *testing.T) {
    // Test: configure with valid URL and token starts poller
    // Test: configure with empty URL stops poller
    // Test: reconfigure while running restarts poller
    // Test: invalid URL returns error
}
```

### Integration Tests

1. **Start AgentMux without env vars**
   - Verify poller not running
   - Run `wsh agentmux-config https://agentmux.asaf.cc token123`
   - Verify poller starts
   - Verify cross-host injection works

2. **Change config at runtime**
   - Start with config A
   - Change to config B via wsh
   - Verify new config is used

3. **Clear config**
   - Start with config
   - Run `wsh agentmux-config clear`
   - Verify poller stops

---

## Security Considerations

1. **Token Handling**
   - Never log tokens
   - Base64 encode in OSC sequence (not for security, for delimiter safety)
   - Token only stored in memory

2. **Rate Limiting**
   - ReconfigureGlobalPoller should have rate limiting (max 1 call/second)
   - Prevents malicious rapid reconfiguration

3. **Validation**
   - Validate URL format before accepting
   - Log config changes (without token) for audit

---

## Migration Path

### Before (Current)
```bash
# Must set before starting AgentMux
export AGENTMUX_URL=https://agentmux.asaf.cc
export AGENTMUX_TOKEN=secret
agentmux
```

### After (New)
```bash
# Option 1: Still works with env vars
export AGENTMUX_URL=https://agentmux.asaf.cc
export AGENTMUX_TOKEN=secret
agentmux

# Option 2: Configure at runtime
agentmux
# In any terminal pane:
wsh agentmux-config https://agentmux.asaf.cc secret

# Option 3: In shell init
# Add to ~/.bashrc or ~/.zshrc:
wsh agentmux-config https://agentmux.asaf.cc "$AGENTMUX_TOKEN"
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `pkg/reactive/poller.go` | Modify | Add ReconfigureGlobalPoller |
| `pkg/blockcontroller/blockcontroller.go` | Modify | Extend OSC handler |
| `cmd/wsh/cmd/agentmuxconfig.go` | Create | New wsh command |
| `docs/agentmux-config.md` | Create | Documentation |

---

## Estimated Complexity

- **poller.go changes:** ~50 lines
- **blockcontroller.go changes:** ~30 lines
- **agentmuxconfig.go:** ~60 lines
- **Documentation:** ~100 lines

**Total:** ~240 lines of code + docs

---

## Open Questions

1. Should we support partial updates (URL only, token only)?
   - **Recommendation:** No, require both for simplicity

2. Should config persist across AgentMux restarts?
   - **Recommendation:** No, use env vars for persistence

3. Should we add a status command (`wsh agentmux-status`)?
   - **Recommendation:** Yes, useful for debugging
