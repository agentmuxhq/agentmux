# Implementation Plan: Reactive Agent Messaging

**Status:** Phase 2 Complete - HTTP API Ready
**Priority:** P0 - Major Feature
**Author:** AgentA
**Date:** 2026-01-15

---

## Executive Summary

Enable real-time, reactive messaging between Claude Code instances running in WaveMux panes. When Agent A sends a message to Agent B, the message is injected directly into Agent B's terminal stdin, causing Claude Code to process it as user input and respond immediately.

This is a **first-in-industry feature** - no existing tool provides true reactive agent-to-agent communication for Claude Code.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              WaveMux                                     │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                  │
│  │   Pane 1    │    │   Pane 2    │    │   Pane 3    │                  │
│  │  Claude A   │    │  Claude B   │    │  Claude C   │                  │
│  │  (AgentA)   │    │  (AgentX)   │    │  (AgentG)   │                  │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘                  │
│         │                  │                  │                          │
│         │ PTY stdin        │ PTY stdin        │ PTY stdin               │
│         │                  │                  │                          │
│  ┌──────┴──────────────────┴──────────────────┴──────┐                  │
│  │                   PTY Manager                      │                  │
│  │           (wavemuxsrv - Go backend)               │                  │
│  └──────────────────────┬────────────────────────────┘                  │
│                         │                                                │
│  ┌──────────────────────┴────────────────────────────┐                  │
│  │              Reactive Message Handler              │                  │
│  │    - Receives injection requests from AgentMux    │                  │
│  │    - Routes to correct pane by agent ID           │                  │
│  │    - Writes to PTY master fd                      │                  │
│  └──────────────────────┬────────────────────────────┘                  │
└─────────────────────────┼───────────────────────────────────────────────┘
                          │
                          │ MCP / WebSocket / Unix Socket
                          │
┌─────────────────────────┴───────────────────────────────────────────────┐
│                            AgentMux                                      │
│  ┌────────────────────────────────────────────────────────────────┐     │
│  │                     Message Router                              │     │
│  │  - Standard mailbox messages (existing)                        │     │
│  │  - NEW: Reactive injection requests                            │     │
│  └────────────────────────────────────────────────────────────────┘     │
│                                                                          │
│  Message Types:                                                          │
│  1. mailbox    - Async, agent reads when ready (existing)               │
│  2. reactive   - Sync injection into running terminal (NEW)             │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Component Breakdown

### 1. AgentMux: New `inject_terminal` Capability

**Location:** AgentMux MCP server
**Purpose:** Accept reactive message requests and forward to WaveMux

#### New MCP Tool: `inject_terminal`

```typescript
{
  name: "inject_terminal",
  description: "Inject a message into a running agent's terminal, causing immediate processing",
  inputSchema: {
    type: "object",
    properties: {
      target_agent: {
        type: "string",
        description: "Agent ID to inject message into (e.g., 'AgentX', 'AgentG')"
      },
      message: {
        type: "string",
        description: "The message to inject as user input"
      },
      priority: {
        type: "string",
        enum: ["normal", "urgent"],
        description: "Urgent messages may interrupt current processing"
      },
      wait_for_idle: {
        type: "boolean",
        default: true,
        description: "Wait for agent to be idle before injecting (recommended)"
      }
    },
    required: ["target_agent", "message"]
  }
}
```

#### Message Flow

```
Agent A calls inject_terminal(target="AgentX", message="Review PR #135")
    │
    ▼
AgentMux receives request
    │
    ├── Validates target agent exists and is online
    ├── Checks agent is in a WaveMux pane (has PTY)
    │
    ▼
AgentMux forwards to WaveMux backend via:
    Option A: WebSocket connection (if WaveMux connects to AgentMux)
    Option B: Unix socket / named pipe
    Option C: HTTP endpoint on wavemuxsrv
    │
    ▼
WaveMux receives injection request
    │
    ├── Looks up pane by WAVEMUX_AGENT_ID
    ├── Gets PTY master file descriptor
    ├── Writes message + newline to PTY stdin
    │
    ▼
Claude Code in target pane receives input
    │
    ├── Processes as user message
    ├── Generates response
    │
    ▼
Response visible in pane (and optionally captured back)
```

---

### 2. WaveMux: PTY Injection Endpoint

**Location:** `pkg/wshutil/` or new `pkg/reactive/`
**Purpose:** Accept injection requests and write to PTY

#### New Go Package: `pkg/reactive/handler.go`

```go
package reactive

import (
    "fmt"
    "sync"
)

// InjectionRequest represents a request to inject text into a terminal
type InjectionRequest struct {
    TargetAgentID string `json:"target_agent"`
    Message       string `json:"message"`
    Priority      string `json:"priority"`
    WaitForIdle   bool   `json:"wait_for_idle"`
    RequestID     string `json:"request_id"`
    SourceAgent   string `json:"source_agent"`
}

// InjectionResponse represents the result of an injection attempt
type InjectionResponse struct {
    Success   bool   `json:"success"`
    RequestID string `json:"request_id"`
    Error     string `json:"error,omitempty"`
    PaneID    string `json:"pane_id,omitempty"`
}

// Handler manages reactive message injection
type Handler struct {
    mu            sync.RWMutex
    agentToPane   map[string]string          // AgentID -> BlockID/PaneID
    paneWriters   map[string]PtyWriter       // PaneID -> PTY write function
}

// PtyWriter is a function that writes to a PTY's stdin
type PtyWriter func(data []byte) error

// RegisterAgent associates an agent ID with a pane
func (h *Handler) RegisterAgent(agentID, paneID string, writer PtyWriter) {
    h.mu.Lock()
    defer h.mu.Unlock()
    h.agentToPane[agentID] = paneID
    h.paneWriters[paneID] = writer
}

// UnregisterAgent removes an agent's registration
func (h *Handler) UnregisterAgent(agentID string) {
    h.mu.Lock()
    defer h.mu.Unlock()
    if paneID, ok := h.agentToPane[agentID]; ok {
        delete(h.paneWriters, paneID)
        delete(h.agentToPane, agentID)
    }
}

// InjectMessage writes a message to the target agent's terminal
func (h *Handler) InjectMessage(req InjectionRequest) InjectionResponse {
    h.mu.RLock()
    paneID, exists := h.agentToPane[req.TargetAgentID]
    if !exists {
        h.mu.RUnlock()
        return InjectionResponse{
            Success:   false,
            RequestID: req.RequestID,
            Error:     fmt.Sprintf("agent %s not found or not in a WaveMux pane", req.TargetAgentID),
        }
    }

    writer, hasWriter := h.paneWriters[paneID]
    h.mu.RUnlock()

    if !hasWriter {
        return InjectionResponse{
            Success:   false,
            RequestID: req.RequestID,
            Error:     fmt.Sprintf("no PTY writer for pane %s", paneID),
        }
    }

    // Format message for Claude Code input
    // Add newline to submit the message
    messageBytes := []byte(req.Message + "\n")

    if err := writer(messageBytes); err != nil {
        return InjectionResponse{
            Success:   false,
            RequestID: req.RequestID,
            Error:     fmt.Sprintf("failed to write to PTY: %v", err),
        }
    }

    return InjectionResponse{
        Success:   true,
        RequestID: req.RequestID,
        PaneID:    paneID,
    }
}
```

#### Integration Point: Shell/PTY Management

The existing PTY management in WaveMux needs to expose write access:

**File:** `pkg/shellexec/shellexec.go` (or similar)

```go
// Add method to get PTY write function for a shell
func (s *ShellProc) GetStdinWriter() func([]byte) error {
    return func(data []byte) error {
        _, err := s.Cmd.Stdin.Write(data)
        return err
    }
}
```

---

### 3. Agent Registration via OSC 16162

When a Claude Code instance starts with `WAVEMUX_AGENT_ID` set, the shell integration already sends this via OSC 16162. WaveMux frontend receives this and updates block metadata.

**Enhancement needed:** Backend must also track agent-to-pane mapping for injection routing.

#### Registration Flow

```
1. Shell starts with WAVEMUX_AGENT_ID=AgentX
2. Shell integration sends: \033]16162;E;{"WAVEMUX_AGENT_ID":"AgentX"}\007
3. WaveMux frontend receives OSC, updates block metadata
4. Frontend notifies backend: "Block abc123 has agent AgentX"
5. Backend registers: agentToPane["AgentX"] = "abc123"
6. Backend stores PTY writer for block abc123
```

#### New WebSocket Message Type

**File:** `pkg/wshrpc/` or similar

```go
// AgentRegistration notifies backend of agent-to-pane mapping
type AgentRegistration struct {
    Type      string `json:"type"`      // "agent_register" or "agent_unregister"
    AgentID   string `json:"agent_id"`
    BlockID   string `json:"block_id"`
    PaneID    string `json:"pane_id"`
}
```

---

### 4. Communication Channel: AgentMux <-> WaveMux

#### Option A: WebSocket (Recommended)

WaveMux backend connects to AgentMux as a client, subscribing to injection requests.

```
AgentMux WebSocket Server (port 8765)
    │
    ├── Agent connections (existing)
    │
    └── WaveMux connection (new)
        - Subscribes to: injection_requests
        - Publishes: injection_responses, agent_online/offline
```

**Pros:** Bidirectional, real-time, existing AgentMux WebSocket infrastructure
**Cons:** Requires WaveMux to maintain connection to AgentMux

#### Option B: HTTP Endpoint on wavemuxsrv

AgentMux calls HTTP endpoint on WaveMux when injection is requested.

```
POST http://localhost:1729/api/reactive/inject
Content-Type: application/json

{
  "target_agent": "AgentX",
  "message": "Please review PR #135",
  "source_agent": "AgentA",
  "request_id": "uuid-here"
}
```

**Pros:** Simple, stateless, easy to implement
**Cons:** Requires WaveMux to expose HTTP endpoint, firewall considerations

#### Option C: Unix Socket / Named Pipe

Direct IPC between AgentMux and WaveMux on same machine.

**Pros:** Fast, no network overhead, secure
**Cons:** Only works locally, more complex setup

**Recommendation:** Start with **Option B (HTTP)** for simplicity, migrate to **Option A (WebSocket)** for production.

---

### 5. Message Formatting and Safety

#### Input Sanitization

Messages must be sanitized before injection to prevent:
- Escape sequence injection (terminal control codes)
- Command injection (if message contains shell metacharacters)
- Excessive length causing buffer issues

```go
func SanitizeMessage(msg string) string {
    // Remove ANSI escape sequences
    ansiRegex := regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]`)
    msg = ansiRegex.ReplaceAllString(msg, "")

    // Remove other control characters (except newline)
    var sanitized strings.Builder
    for _, r := range msg {
        if r == '\n' || (r >= 32 && r < 127) || r > 127 {
            sanitized.WriteRune(r)
        }
    }

    // Limit length
    result := sanitized.String()
    if len(result) > 10000 {
        result = result[:10000] + "\n[Message truncated]"
    }

    return result
}
```

#### Message Envelope (Optional)

For traceability, messages can be wrapped:

```
[Reactive message from AgentA via AgentMux]
Please review PR #135 and provide feedback on the implementation.
[End reactive message]
```

Or simpler, just prepend source:

```
@AgentA: Please review PR #135 and provide feedback.
```

---

## Implementation Phases

### Phase 1: Backend Infrastructure (wavemuxsrv) ✅ COMPLETE

**PR:** [#140](https://github.com/a5af/wavemux/pull/140) - Merged

**Files created:**
- `pkg/reactive/types.go` - Request/response types
- `pkg/reactive/sanitize.go` - Message sanitization with security hardening
- `pkg/reactive/handler.go` - Core injection handler with audit logging
- `pkg/reactive/httphandler.go` - HTTP endpoint handlers

**Files modified:**
- `pkg/web/web.go` - Added reactive API routes
- `cmd/server/main-server.go` - Initialize reactive handler

**Tasks completed:**
1. [x] Create `pkg/reactive/` package
2. [x] Implement `Handler` with agent registration and audit logging
3. [x] Add HTTP endpoints: `/wave/reactive/inject`, `/wave/reactive/agents`, etc.
4. [x] Wire up blockcontroller.SendInput for PTY writes
5. [x] Message sanitization: strips ANSI/OSC/CSI escape sequences
6. [x] Input validation: ValidateAgentID, parseInt overflow protection

**Estimated complexity:** Medium
**Dependencies:** None

### Phase 2: Frontend Agent Tracking ✅ COMPLETE

**PR:** [#141](https://github.com/a5af/wavemux/pull/141) - In Review

**Files modified:**
- `frontend/app/view/term/termwrap.ts` - OSC 16162 handler calls registration API
- `pkg/reactive/httphandler.go` - Added register/unregister endpoints
- `pkg/web/web.go` - Added register/unregister routes

**Tasks completed:**
1. [x] Add HTTP endpoints for registration: POST /wave/reactive/register, /unregister
2. [x] On OSC 16162 "E" command with WAVEMUX_AGENT_ID, register agent with backend
3. [x] Track registered agents per block to detect changes
4. [x] Unregister agent when terminal is disposed
5. [x] Handle agent ID changes (unregister old, register new)

**Estimated complexity:** Low
**Dependencies:** Phase 1

### Phase 3: AgentMux Integration ⏸️ BLOCKED

**Status:** Pending AgentMux source code access

The HTTP API is ready for AgentMux to consume. When AgentMux source becomes available:

**Files to create/modify:**
- `agentmux/src/tools/inject_terminal.ts` - New MCP tool
- `agentmux/src/wavemux_client.ts` - HTTP client for WaveMux

**Tasks:**
1. [ ] Add `inject_terminal` MCP tool
2. [ ] Implement HTTP client to call WaveMux endpoint
3. [ ] Add response handling and error reporting
4. [ ] Update agent list to show injection capability

**Estimated complexity:** Medium
**Dependencies:** Phase 1, AgentMux source access

---

## Using the HTTP API Directly

Until AgentMux integration is complete, agents can call the HTTP API directly using curl or any HTTP client.

### API Endpoints

All endpoints are served by wavemuxsrv on the same port as other wave services (typically 1729).

#### POST /wave/reactive/inject

Inject a message into a target agent's terminal.

```bash
curl -X POST http://localhost:1729/wave/reactive/inject \
  -H "Content-Type: application/json" \
  -d '{
    "target_agent": "AgentX",
    "message": "Please review PR #135",
    "source_agent": "AgentA",
    "priority": "normal"
  }'
```

**Response (success):**
```json
{
  "success": true,
  "request_id": "uuid-here",
  "block_id": "abc123",
  "timestamp": "2026-01-15T10:30:00Z"
}
```

**Response (error):**
```json
{
  "success": false,
  "error": "agent AgentX not found or not in a WaveMux pane"
}
```

#### GET /wave/reactive/agents

List all registered agents.

```bash
curl http://localhost:1729/wave/reactive/agents
```

**Response:**
```json
{
  "agents": [
    {
      "agent_id": "AgentX",
      "block_id": "abc123",
      "tab_id": "tab1",
      "registered_at": "2026-01-15T10:00:00Z",
      "last_seen": "2026-01-15T10:30:00Z"
    }
  ]
}
```

#### GET /wave/reactive/agent?id=AgentX

Get info for a specific agent.

```bash
curl "http://localhost:1729/wave/reactive/agent?id=AgentX"
```

#### POST /wave/reactive/register

Register an agent (called automatically by frontend, but can be used manually).

```bash
curl -X POST http://localhost:1729/wave/reactive/register \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "AgentX",
    "block_id": "abc123",
    "tab_id": "tab1"
  }'
```

#### POST /wave/reactive/unregister

Unregister an agent.

```bash
curl -X POST http://localhost:1729/wave/reactive/unregister \
  -H "Content-Type: application/json" \
  -d '{"agent_id": "AgentX"}'
```

#### GET /wave/reactive/audit?limit=50

Get recent injection audit log entries.

```bash
curl "http://localhost:1729/wave/reactive/audit?limit=10"
```

### Example: Agent-to-Agent Communication via Bash

```bash
# From AgentA's terminal, send message to AgentX
curl -s -X POST http://localhost:1729/wave/reactive/inject \
  -H "Content-Type: application/json" \
  -d '{
    "target_agent": "AgentX",
    "message": "Hello from AgentA! Please acknowledge.",
    "source_agent": "AgentA"
  }' | jq .
```

---

### Phase 4: Testing and Hardening

**Tasks:**
1. [ ] Unit tests for injection handler
2. [ ] Integration test: AgentA injects to AgentX
3. [ ] Security audit: escape sequence injection
4. [ ] Performance test: rapid injection handling
5. [ ] Error handling: target agent offline, PTY closed

**Estimated complexity:** Medium
**Dependencies:** Phases 1-3

### Phase 5: Advanced Features (Future)

1. **Wait for idle** - Detect when Claude Code is waiting for input
2. **Response capture** - Capture and return Claude's response
3. **Acknowledgment** - Target agent confirms receipt
4. **Priority queue** - Urgent messages interrupt lower priority
5. **Broadcast** - Send to multiple agents simultaneously

---

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `pkg/reactive/handler.go` | Core injection handler |
| `pkg/reactive/sanitize.go` | Message sanitization |
| `pkg/reactive/types.go` | Request/response types |
| `pkg/reactive/handler_test.go` | Unit tests |

### Modified Files

| File | Changes |
|------|---------|
| `pkg/wshutil/wshserver.go` | Add HTTP endpoint |
| `pkg/shellexec/shellexec.go` | Expose PTY writer |
| `frontend/app/block/blockframe.tsx` | Agent registration |
| `cmd/wavemuxsrv/main.go` | Initialize reactive handler |

### AgentMux Files (Separate Repo)

| File | Changes |
|------|---------|
| `src/tools/inject_terminal.ts` | New MCP tool |
| `src/wavemux_client.ts` | HTTP client |
| `src/index.ts` | Register new tool |

---

## Security Considerations

### 1. Authentication

- Only registered agents can inject messages
- AgentMux validates source agent identity
- WaveMux validates request comes from trusted AgentMux

### 2. Authorization

- Agents can only inject to agents in same "team" or "workspace"
- Optional: require explicit permission from target agent
- Rate limiting to prevent abuse

### 3. Input Validation

- Sanitize all injected messages
- Block terminal escape sequences
- Limit message size
- Log all injection attempts

### 4. Audit Trail

```go
type InjectionAuditLog struct {
    Timestamp    time.Time
    SourceAgent  string
    TargetAgent  string
    MessageHash  string  // SHA256 of message
    Success      bool
    ErrorMessage string
}
```

---

## Testing Plan

### Unit Tests

```go
func TestSanitizeMessage(t *testing.T) {
    tests := []struct {
        input    string
        expected string
    }{
        {"hello world", "hello world"},
        {"hello\x1b[31mred\x1b[0m", "hellored"},  // Strip ANSI
        {"line1\nline2", "line1\nline2"},         // Keep newlines
        {strings.Repeat("a", 20000), /* truncated */},
    }
    // ...
}

func TestInjectMessage(t *testing.T) {
    handler := NewHandler()

    // Register mock agent
    var written []byte
    handler.RegisterAgent("AgentX", "pane1", func(data []byte) error {
        written = data
        return nil
    })

    // Inject message
    resp := handler.InjectMessage(InjectionRequest{
        TargetAgentID: "AgentX",
        Message:       "Hello from AgentA",
    })

    assert.True(t, resp.Success)
    assert.Equal(t, "Hello from AgentA\n", string(written))
}
```

### Integration Tests

1. **Happy path:** AgentA injects to AgentX, Claude responds
2. **Agent offline:** Target agent not registered, error returned
3. **Rapid injection:** Multiple messages in quick succession
4. **Large message:** Message at/near size limit
5. **Special characters:** Unicode, emoji, quotes in message

### Manual Testing Checklist

- [ ] Start WaveMux with two panes (AgentA, AgentX)
- [ ] From AgentA, call `inject_terminal` to AgentX
- [ ] Verify message appears in AgentX pane
- [ ] Verify Claude Code processes and responds
- [ ] Test with agent offline - verify error
- [ ] Test escape sequence sanitization

---

## Success Metrics

1. **Latency:** < 100ms from inject call to message appearing in target pane
2. **Reliability:** 99.9% successful injection when target is online
3. **Security:** Zero escape sequence injection vulnerabilities
4. **Usability:** Agents can communicate without user intervention

---

## Open Questions

1. **Should responses be captured and returned?**
   - Pro: Enables request/response pattern
   - Con: Complexity, unclear when response "ends"

2. **How to handle target agent mid-response?**
   - Option A: Queue until idle
   - Option B: Inject immediately (may confuse Claude)
   - Option C: Return error, retry later

3. **Multi-WaveMux support?**
   - Currently assumes single WaveMux instance
   - Future: AgentMux routes to correct WaveMux instance

4. **Permission model?**
   - Any agent can inject to any agent?
   - Require explicit opt-in per agent?
   - Team/workspace-based permissions?

---

## References

- [GitHub Issue #2929: Programmatically Drive Claude Instances](https://github.com/anthropics/claude-code/issues/2929)
- [GitHub Issue #4993: Agent-to-Agent Communication](https://github.com/anthropics/claude-code/issues/4993)
- [WaveMux OSC 16162 Shell Integration](../pkg/util/shellutil/shellintegration/)
- [AgentMux MCP Server](../../agentmux/)

---

## Appendix: Example Usage

### From Claude Code (via MCP)

```
Human: Send a message to AgentX asking them to review PR #135