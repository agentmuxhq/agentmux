# Reactive Agent Communication Research

**Date:** 2026-01-15
**Author:** AgentA
**Status:** Research Complete

---

## Executive Summary

This document researches how to implement **reactive communication between Claude Code agents** running in AgentMux terminal panes. The primary use case:

> AgentA finishes code changes → injects message into AgentX's running terminal → AgentX (reviewer) receives notification in real-time

**Key Finding:** AgentMux already has deployed AWS infrastructure and comprehensive specs for this. The recommended approach combines:
1. **Webhook-based terminal injection** (for external events)
2. **MCP server tools** (for agent-to-agent messaging)
3. **AgentMux as message broker** (unified API for both patterns)

---

## Table of Contents

1. [User Scenario](#1-user-scenario)
2. [Critical Insight: Claude Code is NOT a REPL](#2-critical-insight-claude-code-is-not-a-repl)
3. [Existing AgentMux Infrastructure](#3-existing-agentmux-infrastructure)
4. [Terminal Injection Methods](#4-terminal-injection-methods)
5. [MCP Protocol for Agent Communication](#5-mcp-protocol-for-agent-communication)
6. [AgentMux Integration Architecture](#6-agentmux-integration-architecture)
7. [Implementation Options](#7-implementation-options)
8. [Security Considerations](#8-security-considerations)
9. [Recommended Architecture](#9-recommended-architecture)
10. [Sources](#10-sources)

---

## 1. User Scenario

### The Problem

Multiple Claude Code agents work on the same codebase:
- **AgentA** (area54) - Lead developer, writes features
- **AgentX** (claudius) - Code reviewer
- **AgentY**, **AgentG**, etc. - Other specialists

**Current workflow (manual):**
1. AgentA finishes work, pushes to GitHub
2. AgentA manually tells user "ready for review"
3. User manually tells AgentX "review PR #123"
4. AgentX reviews and provides feedback
5. Repeat...

**Desired workflow (reactive):**
1. AgentA finishes work, pushes to GitHub
2. AgentA calls `agentmux.send_message("agentx", "PR #123 ready for review")`
3. AgentX's terminal receives: `[AGENT-MSG] AgentA: PR #123 ready for review`
4. AgentX sees notification in real-time, begins review
5. AgentX calls `agentmux.send_message("agenta", "Changes requested on line 42")`
6. AgentA receives notification immediately

### Requirements

| Requirement | Description |
|-------------|-------------|
| **Real-time** | Messages appear within seconds |
| **Bidirectional** | Any agent can message any other agent |
| **Persistent identity** | Agents have stable IDs across sessions |
| **Terminal injection** | Messages appear in terminal output |
| **Optional: Command execution** | Optionally trigger shell commands |

---

## 2. Critical Insight: Claude Code IS a REPL

### Why This Matters

**Claude Code is a persistent interactive REPL (Read-Eval-Print-Loop).**

- Running process maintains full conversation context
- Continuously reads from stdin (terminal PTY)
- User types message → Claude processes → Claude responds → loop continues
- Session persists until explicitly exited

### Implications

| Approach | Works? | Why |
|----------|--------|-----|
| Inject into terminal PTY | ✅ Yes | Claude sees it as user input, responds |
| Send via MCP tool call | ✅ Yes | Claude invokes tools, receives responses |
| Write to shared file | ✅ Yes | Claude can read files |
| ANSI notification (output only) | ⚠️ Partial | Claude sees it but won't auto-respond |

### The Solution

**Direct PTY injection - Claude receives as user input:**
- Inject text into the terminal PTY where Claude Code is running
- Claude Code sees injected text as a new user message
- Claude Code processes and responds to it automatically

```
┌─────────────────────────────────────────────────┐
│ Terminal Pane (AgentX) - Claude Code running    │
│                                                 │
│ Human: review the PR                            │
│ Claude: I'll review PR #123...                  │
│                                                 │
│ Human: [INJECTED] AgentA: PR updated, L42 fixed│  ← Injected as input!
│ Claude: Thanks for the update. Let me re-check │  ← Claude responds!
│         line 42...                              │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Two Injection Modes

| Mode | How | Result |
|------|-----|--------|
| **Input injection** | Write to PTY stdin | Claude sees as user message, responds |
| **Output injection** | Write to PTY stdout | Appears in terminal, Claude must manually read |

**Input injection is preferred** - triggers automatic Claude response.

---

## 3. Existing AgentMux Infrastructure

### Already Deployed (AWS)

| Component | Endpoint | Status |
|-----------|----------|--------|
| Lambda Router | `agentmux-webhook-router-prod` | ✅ Deployed |
| HTTP API | `https://m6jrh0uo28.execute-api.us-east-1.amazonaws.com` | ✅ Live |
| WebSocket API | `wss://oft9nfu83k.execute-api.us-east-1.amazonaws.com` | ✅ Live |
| DynamoDB Config | `AgentMuxWebhookConfig-prod` | ✅ Active |
| DynamoDB Connections | `AgentMuxConnections-prod` | ✅ Active |

### Existing Specifications

| Spec | Location | Purpose |
|------|----------|---------|
| `SPEC_REACTIVE_AGENT_COMMUNICATION.md` | docs/specs/ | Webhook → terminal injection |
| `SPEC_AGENTMUX_INTEGRATION.md` | docs/specs/ | MCP server for agent messaging |
| `SPEC_WEBHOOK_DELIVERY_README.md` | docs/specs/ | Phase 2 implementation guide |

### What's Missing

| Component | Status |
|-----------|--------|
| Go webhook client in agentmuxsrv | ⏳ Not implemented |
| MCP server integration | ⏳ Not implemented |
| AgentMux ↔ AgentMux bridge | ⏳ Not implemented |
| Frontend configuration UI | ⏳ Not implemented |

---

## 4. Terminal Injection Methods

### Method 1: tmux send-keys (Established Pattern)

**How it works:** Simulates keystrokes into a tmux pane

```bash
# Send text to a specific pane
tmux send-keys -t session:window.pane "echo 'Hello AgentX'" Enter

# Send to all panes (broadcast)
tmux list-panes -a -F "#{session_name}:#{window_index}.#{pane_index}" | \
  while read pane; do
    tmux send-keys -t "$pane" "[MSG] Hello" Enter
  done
```

**Pros:** Simple, reliable, widely supported
**Cons:** Requires tmux, not directly applicable to AgentMux/Electron

### Method 2: PTY Master Injection (Low-Level)

**How it works:** Write directly to PTY master file descriptor

```
PTY Pair:
  Master FD ──write──→ Slave FD (terminal reads from here)

Process with Master FD can inject any text into terminal.
```

**AgentMux Implementation:**
- AgentMux owns PTY master for each terminal pane
- Can write ANSI text directly via `BlockController`
- Already used for shell integration (OSC sequences)

### Method 3: WebSocket + Terminal Bridge (AgentMux Approach)

**How it works:**
1. External event → AWS API Gateway
2. Lambda routes to WebSocket connection
3. AgentMux client receives message
4. Client injects into terminal PTY

```
GitHub Webhook
     ↓
AWS Lambda
     ↓
WebSocket API
     ↓
AgentMux Client ──PTY write──→ Terminal Pane
```

### Method 4: ANSI Text Injection (Universal)

**How it works:** Printf formatted text with ANSI escape codes

```bash
# Colored notification
printf "\033[36m[AGENT-MSG]\033[0m \033[33mAgentA\033[0m: PR ready for review\n"

# With timestamp
printf "\033[90m[%s]\033[0m \033[36m[AGENT-MSG]\033[0m %s: %s\n" \
  "$(date +%H:%M:%S)" "AgentA" "PR #123 ready"
```

**Output:**
```
[15:30:42] [AGENT-MSG] AgentA: PR #123 ready
```

**Pros:** Works everywhere, human readable, machine parseable
**Cons:** No structured data, text only

### Comparison Table

| Method | Latency | Setup | AgentMux Support | Best For |
|--------|---------|-------|-----------------|----------|
| tmux send-keys | <10ms | Simple | ❌ Not applicable | tmux users |
| PTY Master | <1ms | Complex | ✅ Native | Low-level control |
| WebSocket Bridge | 100-500ms | AWS setup | ✅ Spec exists | External events |
| ANSI Text | <10ms | Simple | ✅ Trivial | Notifications |

---

## 5. MCP Protocol for Agent Communication

### What is MCP?

**Model Context Protocol (MCP)** is an open standard by Anthropic for AI ↔ tool communication.

- JSON-RPC 2.0 based
- Type-safe tool definitions
- Bidirectional messaging
- Adopted by OpenAI, Microsoft, and others (2025)

### MCP for Agent-to-Agent Messaging

**Key insight:** Agents can be both MCP clients AND servers.

```
Agent A (MCP Client)           Agent B (MCP Server)
    │                               │
    │  ──── tool call ────────────→ │
    │                               │
    │  ←─── response ───────────── │
    │                               │
```

### Proposed MCP Tools for AgentMux

From `SPEC_AGENTMUX_INTEGRATION.md`:

```typescript
// Tool 1: Direct message to specific agent
waveterm_send_message({
  to: "agentx",           // Target agent ID
  message: "PR ready",    // Message content
  priority?: "high",      // Optional priority
  metadata?: {...}        // Optional structured data
})

// Tool 2: Broadcast to all agents
waveterm_broadcast({
  message: "Build complete",
  type?: "success"
})

// Tool 3: List active agents
waveterm_list_agents({
  includeInactive?: boolean
})
```

### MCP vs A2A (Google's Agent-to-Agent Protocol)

| Protocol | Purpose | Use Case |
|----------|---------|----------|
| **MCP** | Agent ↔ Tools | Claude Code calling AgentMux tools |
| **A2A** | Agent ↔ Agent | Direct peer-to-peer messaging |

**Recommendation:** Use MCP for tool invocation, extend for agent messaging via AgentMux.

---

## 6. AgentMux Integration Architecture

### Current AgentMux Capabilities

AgentMux already provides:
- `send_message(to, message)` - Direct messaging
- `broadcast_message(message)` - Broadcast to all
- `list_agents()` - Agent discovery
- `read_messages()` - Inbox polling

### The Gap: Terminal Injection

AgentMux sends messages to an inbox. But agents need messages **injected into their terminal** for real-time visibility.

### Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         AgentMux Server                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Message API │  │ Agent       │  │ Terminal Injection API  │  │
│  │ (existing)  │  │ Registry    │  │ (NEW)                   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      AgentMux Application                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ WebSocket   │  │ Terminal    │  │ PTY                     │  │
│  │ Client      │──│ Router      │──│ Injector                │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│               Terminal Pane (Claude Code running)                │
│                                                                  │
│  $ claude "implement feature X"                                  │
│  Working on feature X...                                         │
│                                                                  │
│  [AGENT-MSG] AgentA: PR #123 merged, pull latest               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Two Delivery Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Inbox** | Store-and-forward, agent polls | Async, offline agents |
| **Injection** | Real-time terminal injection | Active agents, urgent |

### AgentMux API Extension

```typescript
// Existing: Store in inbox
agentmux.send_message({
  to: "agentx",
  message: "PR ready"
})

// NEW: Inject into terminal (real-time)
agentmux.inject_terminal({
  to: "agentx",
  message: "PR ready",
  format: "ansi",  // or "plain", "json"
  priority: "high"
})
```

---

## 7. Implementation Options

### Option A: AgentMux-Only (Simplest)

**Architecture:** AgentMux handles both messaging AND terminal injection

```
Agent A                    AgentMux                   Agent X Terminal
   │                          │                            │
   │ ─inject_terminal()────→  │                            │
   │                          │ ─WebSocket────────────────→│
   │                          │                            │ (message appears)
```

**Pros:**
- Single integration point
- AgentMux already knows agent identities
- Centralized routing logic

**Cons:**
- AgentMux needs AgentMux-specific code
- Tight coupling

### Option B: AgentMux MCP Server (Most Flexible)

**Architecture:** AgentMux exposes MCP tools, AgentMux routes to them

```
Agent A                    AgentMux                   AgentMux (MCP)
   │                          │                            │
   │ ─send_to_terminal()───→  │                            │
   │                          │ ─MCP tool call────────────→│
   │                          │                            │ (inject to PTY)
```

**Pros:**
- Standard MCP protocol
- AgentMux controls its own injection logic
- Decoupled architecture

**Cons:**
- More moving parts
- MCP server implementation needed in AgentMux

### Option C: Hybrid (Recommended)

**Architecture:** AgentMux for routing, AgentMux for injection, both connected

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                  │
│  Agent A ──MCP──→ AgentMux ──WebSocket──→ AgentMux ──PTY──→ Term │
│                                                                  │
│  Agent X ──MCP──→ AgentMux ──WebSocket──→ AgentMux ──PTY──→ Term │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Flow:**
1. AgentA calls `agentmux.inject_terminal("agentx", "message")`
2. AgentMux looks up AgentX's AgentMux connection
3. AgentMux sends via WebSocket to AgentMux
4. AgentMux injects into AgentX's terminal PTY

**Pros:**
- Clear separation of concerns
- AgentMux handles identity/routing
- AgentMux handles terminal I/O
- Leverages existing AWS infrastructure

---

## 8. Security Considerations

### Authentication

| Layer | Mechanism |
|-------|-----------|
| AgentMux | Token-based auth (AGENTMUX_TOKEN) |
| AgentMux WebSocket | Workspace-scoped tokens |
| Terminal injection | Agent-to-pane mapping verification |

### Authorization

- Agents can only message agents in same workspace
- Rate limiting: 100 messages/minute per agent
- Message size limit: 4KB

### Injection Safety

**DO NOT** allow arbitrary command execution. Use templates:

```typescript
// SAFE: Template with variable substitution
template: "echo '[AGENT-MSG] {{from}}: {{message}}'"

// UNSAFE: Direct command execution
command: userInput  // ❌ Never do this
```

### Audit Logging

```json
{
  "timestamp": "2026-01-15T15:30:00Z",
  "from": "agenta",
  "to": "agentx",
  "message_type": "terminal_injection",
  "message_preview": "PR #123 ready...",
  "delivered": true
}
```

---

## 9. Recommended Architecture

### Phase 1: AgentMux Terminal Injection API

**Add to AgentMux server:**

```typescript
// New MCP tool
inject_terminal({
  to: string,           // Target agent ID
  message: string,      // Message content
  format?: "ansi" | "plain",
  priority?: "normal" | "high" | "urgent"
})
```

**AgentMux server changes:**
1. Maintain agent → AgentMux connection mapping
2. On inject_terminal, lookup target's AgentMux WebSocket
3. Send injection request via WebSocket
4. Return delivery confirmation

### Phase 2: AgentMux WebSocket Client

**Add to agentmuxsrv:**

```go
// pkg/agentmux/client.go
type AgentMuxClient struct {
    conn     *websocket.Conn
    agentID  string
    handlers map[string]MessageHandler
}

func (c *AgentMuxClient) OnTerminalInjection(msg InjectionMessage) {
    pane := c.findPaneForAgent(msg.To)
    pane.InjectANSI(msg.Format())
}
```

### Phase 3: Agent Registration

**On AgentMux startup:**
1. Read WAVEMUX_AGENT_ID from environment
2. Connect to AgentMux WebSocket
3. Register: `{agentId, agentmuxInstanceId, paneIds}`
4. Heartbeat every 30s

**On terminal pane open:**
1. Update registration with new pane ID
2. AgentMux can now route to this pane

### Message Flow (Complete)

```
1. AgentA (Claude Code) calls:
   agentmux.inject_terminal("agentx", "PR ready")

2. AgentMux server:
   - Validates AgentA's token
   - Looks up AgentX's AgentMux connection
   - Sends: {type: "inject", to: "agentx", message: "PR ready"}

3. AgentMux (AgentX's instance):
   - Receives WebSocket message
   - Finds AgentX's active terminal pane
   - Writes to PTY: "\033[36m[AGENT-MSG]\033[0m AgentA: PR ready\n"

4. AgentX's terminal shows:
   [AGENT-MSG] AgentA: PR ready

5. AgentX (Claude Code) sees message on next prompt
```

---

## 10. Sources

### Official Documentation
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [Claude Code CLI Reference](https://www.eesel.ai/blog/claude-code-cli-reference)
- [Claude Code Subagents](https://code.claude.com/docs/en/sub-agents)

### Research Papers
- [Survey of Agent Interoperability Protocols (arXiv)](https://arxiv.org/html/2505.02279v1)

### Technical References
- [tmux send-keys Command](https://tmuxai.dev/tmux-send-keys/)
- [tmux Scripting Guide](https://tao-of-tmux.readthedocs.io/)
- [PTY Manual (Linux)](https://man7.org/linux/man-pages/man7/pty.7.html)
- [ANSI Escape Codes (2025)](https://jvns.ca/blog/2025/03/07/escape-code-standards/)

### Protocol Comparisons
- [MCP vs A2A Explained](https://www.clarifai.com/blog/mcp-vs-a2a-clearly-explained)
- [MCP vs A2A Guide (Auth0)](https://auth0.com/blog/mcp-vs-a2a/)

### Multi-Agent Frameworks
- [CrewAI Guide](https://mem0.ai/blog/crewai-guide-multi-agent-ai-teams)
- [mcp-agent (GitHub)](https://github.com/lastmile-ai/mcp-agent)

### Keynotes
- [Claude Code: Single Agent to Multi-Agent Systems (AIware 2025)](https://2025.aiwareconf.org/details/aiware-2025-keynotes/1/Claude-Code-From-Single-Agent-in-Terminal-to-Multi-Agent-Systems)

---

## Appendix A: Quick Reference

### ANSI Message Format

```bash
# Standard notification
printf "\033[36m[AGENT-MSG]\033[0m \033[33m%s\033[0m: %s\n" "$FROM" "$MSG"

# With timestamp
printf "\033[90m[%s]\033[0m \033[36m[AGENT-MSG]\033[0m \033[33m%s\033[0m: %s\n" \
  "$(date +%H:%M:%S)" "$FROM" "$MSG"

# Priority: urgent (red)
printf "\033[31m[URGENT]\033[0m \033[33m%s\033[0m: %s\n" "$FROM" "$MSG"
```

### AgentMux Tool Calls

```typescript
// Send to inbox (async)
mcp__agentmux__send_message({
  to: "agentx",
  message: "PR ready for review"
})

// Inject to terminal (real-time) - PROPOSED
mcp__agentmux__inject_terminal({
  to: "agentx",
  message: "PR ready for review",
  format: "ansi"
})

// Broadcast to all
mcp__agentmux__broadcast_message({
  message: "Build complete"
})
```

---

## Appendix B: Existing AgentMux Specs

| Spec | Purpose | Status |
|------|---------|--------|
| `SPEC_REACTIVE_AGENT_COMMUNICATION.md` | Webhook → terminal injection | AWS deployed |
| `SPEC_AGENTMUX_INTEGRATION.md` | MCP server design | Spec complete |
| `SPEC_WEBHOOK_DELIVERY_README.md` | Implementation guide | Ready |

---

*End of Research Document*
