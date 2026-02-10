# Claude Code Wrapper — Technical Specification

**Status:** Draft
**Author:** Agent3
**Date:** 2026-02-10
**Depends on:** Unified AI Pane (Phase A, PRs #228, #234, #237)

---

## 1. Overview

AgentMux's Unified AI Pane currently spawns Claude Code as a raw subprocess and parses its NDJSON output via the adapter layer (`adapters.rs` → `process.rs` → `agent.rs`). The next step is making it a **polished wrapper** — users interact with a fully skinned AgentMux experience while Claude Code runs underneath. The raw Claude Code terminal UI is never shown.

### Goals

1. **Transparent wrapping** — Users never see the Claude Code TUI. All interaction happens through the unified AI pane.
2. **Full streaming** — Token-by-token text streaming, tool use progress, thinking/reasoning blocks, all rendered live.
3. **Tool approval UI** — Approve, deny, or edit destructive tool operations from the AgentMux UI.
4. **Multi-turn conversations** — Send follow-up messages within the same session without restarting the subprocess.
5. **Session persistence** — Resume previous Claude Code sessions across app restarts.
6. **Pane awareness** — Claude Code can see terminal scrollback, editor content, and other panes via an MCP server.
7. **Cost visibility** — Display per-message and cumulative token usage and USD cost.

### Non-Goals

- Replacing Claude Code's internal logic (we run it as-is)
- Supporting Claude Code's interactive TUI mode (we only use `-p` mode)
- Implementing our own Anthropic API client (Claude Code handles all API calls)

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   AgentMux Frontend                      │
│                                                          │
│  UnifiedAIView ◄── messages atom ◄── applyAdapterEvent  │
│       │                                    ▲             │
│       │ (user input)                       │             │
│       ▼                                    │             │
│  agent-api.ts ──► Tauri IPC ──► agent.rs ──┘             │
│                                    │                     │
│                                    ▼                     │
│                              process.rs                  │
│                                    │                     │
│                                    ▼                     │
│                            ┌──────────────┐              │
│                            │ Claude Code  │              │
│                            │ subprocess   │              │
│                            │              │              │
│                            │ -p           │              │
│                            │ --output-format stream-json │
│                            │ --verbose    │              │
│                            │ --include-partial-messages  │
│                            │ --input-format stream-json  │
│                            │ --mcp-config (pane MCP)     │
│                            └──────────────┘              │
└─────────────────────────────────────────────────────────┘
```

### Current State (Phase A)

The adapter pipeline is already built and working:

| Layer | File | Status |
|-------|------|--------|
| Rust types | `backend/ai/unified.rs` | Done (780 lines) |
| Rust agent state | `backend/ai/agent.rs` | Done (594 lines) |
| Rust adapters | `backend/ai/adapters.rs` | Done (981 lines) |
| Rust process mgmt | `backend/ai/process.rs` | Done (615 lines) |
| Tauri IPC commands | `commands/agent.rs` | Done (421 lines) |
| TS types | `unifiedai/unified-types.ts` | Done (530 lines) |
| TS API bridge | `unifiedai/agent-api.ts` | Done (139 lines) |
| TS state/hooks | `unifiedai/useUnifiedAI.ts` | Done (303 lines) |
| TS ViewModel | `unifiedai/unifiedai-model.ts` | Done (260 lines) |
| React view | `unifiedai/unifiedai-view.tsx` | Done (420 lines) |
| Styles | `unifiedai/unifiedai.scss` | Done (520 lines) |

### What This Spec Adds

| Component | Location | Purpose |
|-----------|----------|---------|
| Enhanced NDJSON adapter | `adapters.rs` | Parse full Claude Code SDK protocol (6 event types) |
| Multi-turn stdin writer | `process.rs` | `--input-format stream-json` for follow-up messages |
| Tool approval bridge | `agent.rs` + frontend | Bidirectional approval flow |
| Session management | `agent.rs` + `wstore` | Persist/resume session IDs |
| MCP pane server | `backend/mcp/` (new) | Pane awareness tools for Claude Code |
| Enhanced view | `unifiedai-view.tsx` | Tool approval UI, cost display, session controls |

---

## 3. Claude Code Protocol Reference

### 3.1 Invocation

```bash
claude -p \
  --output-format stream-json \
  --verbose \
  --include-partial-messages \
  --input-format stream-json \
  --mcp-config /tmp/agentmux-mcp-{pane_id}.json \
  --allowedTools "mcp__agentmux__*" \
  "initial prompt text"
```

### 3.2 Environment Variables

Set on the subprocess:

| Variable | Value | Purpose |
|----------|-------|---------|
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | `1` | Disables autoupdater, telemetry, error reporting |
| `DISABLE_AUTOUPDATER` | `1` | Redundant but explicit |
| `DISABLE_TELEMETRY` | `1` | No Statsig telemetry |
| `CLAUDE_CODE_DISABLE_TERMINAL_TITLE` | `1` | Don't change terminal title |
| `NODE_OPTIONS` | `--max-old-space-size=4096` | Prevent OOM on long sessions |
| `ANTHROPIC_API_KEY` | (from user settings) | API key passthrough |
| `ANTHROPIC_MODEL` | (from user settings) | Model override (optional) |

### 3.3 NDJSON Event Types

Claude Code with `--output-format stream-json` emits 6 top-level event types:

#### `system` (init)
First event. Contains session metadata.
```json
{
  "type": "system",
  "subtype": "init",
  "session_id": "abc-123",
  "cwd": "/home/user/project",
  "model": "claude-sonnet-4-5-20250514",
  "tools": ["Read", "Write", "Bash", "Glob", "Grep", ...],
  "mcp_servers": [{"name": "agentmux", "status": "connected"}],
  "permissionMode": "default"
}
```

#### `system` (compact_boundary)
Emitted when conversation history is compacted.
```json
{
  "type": "system",
  "subtype": "compact_boundary",
  "session_id": "abc-123"
}
```

#### `stream_event`
Token-level streaming (only with `--include-partial-messages`). Contains an inner Anthropic API raw streaming event.
```json
{
  "type": "stream_event",
  "session_id": "abc-123",
  "event": {
    "type": "content_block_delta",
    "index": 0,
    "delta": { "type": "text_delta", "text": "Hello" }
  }
}
```

Inner event types: `message_start`, `content_block_start`, `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`.

**Note:** Streaming events are NOT emitted when extended thinking (`maxThinkingTokens`) is explicitly set.

#### `assistant`
Complete assistant message (emitted after all stream events for a turn).
```json
{
  "type": "assistant",
  "session_id": "abc-123",
  "message": {
    "role": "assistant",
    "content": [
      { "type": "text", "text": "Full response text" },
      { "type": "tool_use", "id": "toolu_abc", "name": "Bash", "input": {"command": "ls"} }
    ],
    "model": "claude-sonnet-4-5-20250514",
    "usage": { "input_tokens": 500, "output_tokens": 200 }
  }
}
```

Content block types: `text`, `thinking` (with `thinking` + `signature` fields), `tool_use` (with `id`, `name`, `input`).

#### `user` (tool results)
Emitted after tool execution, contains tool results.
```json
{
  "type": "user",
  "session_id": "abc-123",
  "message": {
    "role": "user",
    "content": [
      {
        "type": "tool_result",
        "tool_use_id": "toolu_abc",
        "content": "file1.txt\nfile2.txt\n",
        "is_error": false
      }
    ]
  }
}
```

#### `result`
Final event with cost and usage statistics.
```json
{
  "type": "result",
  "subtype": "success",
  "session_id": "abc-123",
  "is_error": false,
  "duration_ms": 15234,
  "duration_api_ms": 12100,
  "num_turns": 3,
  "result": "Final text response",
  "total_cost_usd": 0.042,
  "usage": {
    "input_tokens": 1500,
    "output_tokens": 800,
    "cache_read_tokens": 200,
    "cache_write_tokens": 0
  }
}
```

### 3.4 Input via stdin (`--input-format stream-json`)

To send follow-up messages without restarting the subprocess:

```json
{"type":"user","message":{"role":"user","content":"Follow-up question"},"session_id":"abc-123"}
```

The `session_id` must match the one received in the `system` init event.

### 3.5 Session Resume

To resume a previous session:
```bash
claude -p --resume abc-123 --output-format stream-json ...
```

Session data is stored by Claude Code in `~/.claude/projects/` keyed by working directory.

---

## 4. Implementation Plan

### Phase W-1: Enhanced NDJSON Adapter

**Goal:** Parse all 6 Claude Code event types (currently only parsing `stream_event` inner events).

#### Changes to `adapters.rs`

The current `ClaudeCodeEvent` enum handles `system`, `stream_event`, and `result`. It needs to also handle `assistant` and `user` event types.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeCodeEvent {
    #[serde(rename = "system")]
    System {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        model: String,
        #[serde(default)]
        tools: Vec<String>,
        #[serde(default)]
        cwd: String,
    },

    #[serde(rename = "stream_event")]
    StreamEvent {
        #[serde(default)]
        session_id: String,
        event: ClaudeCodeStreamEvent,
    },

    #[serde(rename = "assistant")]
    Assistant {
        #[serde(default)]
        session_id: String,
        message: ClaudeCodeMessage,
    },

    #[serde(rename = "user")]
    User {
        #[serde(default)]
        session_id: String,
        message: ClaudeCodeMessage,
    },

    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        duration_ms: i64,
        #[serde(default)]
        num_turns: i32,
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default)]
        result: Option<serde_json::Value>,
        #[serde(default)]
        usage: Option<ClaudeCodeUsage>,
    },
}
```

New adapter function:

```rust
pub fn adapt_claude_code_event(event: &ClaudeCodeEvent) -> Vec<AdapterEvent> {
    match event {
        ClaudeCodeEvent::System { session_id, model, .. } => {
            // Emit session metadata (new AdapterEvent variant)
            vec![AdapterEvent::SessionStart {
                session_id: session_id.clone(),
                model: if model.is_empty() { None } else { Some(model.clone()) },
            }]
        }
        ClaudeCodeEvent::StreamEvent { event, .. } => {
            adapt_claude_code_stream_event(event)
        }
        ClaudeCodeEvent::Assistant { message, .. } => {
            // Use assistant message as authoritative source
            // (reconcile with streaming if needed)
            adapt_claude_code_assistant_message(message)
        }
        ClaudeCodeEvent::User { message, .. } => {
            // Extract tool results
            adapt_claude_code_user_message(message)
        }
        ClaudeCodeEvent::Result { total_cost_usd, usage, is_error, .. } => {
            // Emit final cost/usage event
            vec![AdapterEvent::SessionEnd {
                total_cost_usd: *total_cost_usd,
                usage: usage.clone().map(|u| u.into()),
                is_error: *is_error,
            }]
        }
    }
}
```

#### New AdapterEvent Variants

```rust
pub enum AdapterEvent {
    // ... existing variants ...

    /// Session started (from Claude Code system init event).
    #[serde(rename = "session_start")]
    SessionStart {
        session_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },

    /// Session ended (from Claude Code result event).
    #[serde(rename = "session_end")]
    SessionEnd {
        total_cost_usd: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
        is_error: bool,
    },
}
```

#### Changes to `process.rs`

The NDJSON reader currently parses `ClaudeCodeStreamEvent` directly from each line. It needs to parse `ClaudeCodeEvent` (the outer wrapper) instead:

```rust
// Current: parse inner stream event only
let event: ClaudeCodeStreamEvent = serde_json::from_str(&line)?;
let adapter_events = adapt_claude_code_stream_event(&event);

// New: parse outer event, dispatch to appropriate adapter
let event: ClaudeCodeEvent = serde_json::from_str(&line)?;
let adapter_events = adapt_claude_code_event(&event);
```

#### Reconciliation Strategy

Both `stream_event` (token-level) and `assistant` (complete message) events arrive for the same turn. Strategy:

1. **During streaming:** Apply `stream_event` inner events in real-time for live updates.
2. **On `assistant` event:** If the final complete message differs from accumulated streaming (e.g., due to dropped events), replace the message parts with the authoritative `assistant` content. This is a no-op in the happy path.
3. **On `result` event:** Mark conversation as complete, record cost.

### Phase W-2: Multi-Turn Stdin

**Goal:** Support follow-up messages within the same subprocess.

#### Changes to `process.rs`

Add a method to send structured NDJSON input:

```rust
impl AgentProcess {
    /// Send a follow-up message to a running Claude Code subprocess.
    ///
    /// Uses the `--input-format stream-json` protocol.
    pub async fn send_user_message(
        &mut self,
        session_id: &str,
        text: &str,
    ) -> Result<(), AgentProcessError> {
        let msg = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": text
            },
            "session_id": session_id
        });
        let line = format!("{}\n", serde_json::to_string(&msg).unwrap());
        self.write_stdin_raw(line.as_bytes()).await
    }
}
```

#### Changes to `agent.rs`

The `send_agent_input` command currently writes raw text to stdin. For Claude Code backends, it should use the structured format:

```rust
#[tauri::command]
async fn send_agent_input(
    request: AgentInputRequest,
    // ...
) -> Result<(), String> {
    // If backend is claudecode and we have a session_id, use structured input
    if agent_info.backend_id == "claudecode" {
        if let Some(session_id) = &agent_info.session_id {
            process.send_user_message(session_id, &request.text.unwrap()).await?;
            return Ok(());
        }
    }
    // Fallback: raw stdin write (for other backends)
    process.write_stdin(request.text.unwrap().as_bytes()).await?;
    Ok(())
}
```

#### Changes to `unifiedai-model.ts`

The ViewModel `sendMessage()` method currently creates a user message and sends raw text. For follow-up messages, it should work the same way from the user's perspective — the Rust layer handles the protocol difference.

No frontend changes needed for this phase.

### Phase W-3: Tool Approval UI

**Goal:** When Claude Code encounters a tool that requires approval (Bash with destructive commands, file writes, etc.), the user sees an approval prompt in the AgentMux UI.

#### Approach: `--permission-prompt-tool`

Claude Code supports delegating permission decisions to an external MCP tool:

```bash
claude -p \
  --permission-prompt-tool mcp__agentmux__approve_tool \
  ...
```

When Claude Code needs approval, it calls this MCP tool with:
```json
{
  "tool_name": "Bash",
  "input": {"command": "rm -rf /tmp/stuff"}
}
```

The tool must respond with:
```json
{"behavior": "allow", "updatedInput": {"command": "rm -rf /tmp/stuff"}}
```
or:
```json
{"behavior": "deny", "message": "User denied this operation"}
```

This is cleaner than intercepting stdin/stdout because Claude Code handles the waiting internally.

#### MCP Tool Implementation

The `approve_tool` MCP tool is part of the pane awareness MCP server (Phase W-5). When invoked:

1. **Rust MCP server** receives the approval request.
2. **Emits a Tauri event** `agent-approval:{pane_id}` with the tool name and input.
3. **Frontend** shows an approval dialog (inline in the tool block).
4. **User** clicks Approve / Deny / Edit.
5. **Frontend** sends the decision back via a Tauri command `resolve_tool_approval`.
6. **Rust** responds to the MCP tool call with the user's decision.
7. **Claude Code** proceeds or aborts based on the response.

#### Frontend Approval UI

In `unifiedai-view.tsx`, the `ToolUsePartView` component gains an approval state:

```tsx
function ToolUsePartView({ part }: { part: ToolUsePart }) {
    // ... existing collapse/expand logic ...

    if (part.approval === "pending") {
        return (
            <div className="uai-tool">
                <div className="uai-tool-line">
                    {/* tool name and summary */}
                </div>
                <div className="uai-tool-approval">
                    <div className="uai-tool-approval-header">
                        Claude wants to use <strong>{part.name}</strong>
                    </div>
                    <pre className="uai-tool-approval-input">
                        {JSON.stringify(part.input, null, 2)}
                    </pre>
                    <div className="uai-tool-approval-actions">
                        <button onClick={handleApprove}>Approve</button>
                        <button onClick={handleDeny}>Deny</button>
                        <button onClick={handleAlwaysAllow}>Always allow</button>
                    </div>
                </div>
            </div>
        );
    }
    // ... normal rendering ...
}
```

#### New Tauri Commands

```rust
#[tauri::command]
async fn resolve_tool_approval(
    pane_id: String,
    call_id: String,
    decision: String,       // "allow" | "deny"
    updated_input: Option<serde_json::Value>,
) -> Result<(), String> { ... }
```

### Phase W-4: Session Management

**Goal:** Persist Claude Code session IDs so conversations can be resumed after subprocess exit or app restart.

#### Session Storage

Add to the agent registry (persisted in WaveStore):

```rust
pub struct AgentInstance {
    // ... existing fields ...
    pub claude_session_id: Option<String>,
    pub claude_cwd: Option<String>,
}
```

When the `system` init event arrives with a `session_id`, store it. When spawning with `--resume`, pass the stored session ID.

#### Resume Flow

1. User opens an AI pane that has a previous `claude_session_id`.
2. Status bar shows "[Resume session]" button.
3. On click, spawn Claude Code with `--resume {session_id}`.
4. Claude Code loads the previous conversation context.
5. User continues the conversation naturally.

#### Auto-Resume

When a pane is restored (e.g., on app launch), if it had an active Claude Code session:
- Show the previous messages (stored in the pane's message history).
- Offer a "Resume" button instead of auto-starting (to avoid unnecessary API costs).

### Phase W-5: MCP Pane Awareness Server

**Goal:** Claude Code can see terminal scrollback, editor content, and other pane state via MCP tools.

#### MCP Server Architecture

AgentMux starts a local TCP MCP server (JSON-RPC 2.0) for each agent subprocess. The server config is written to a temp file and passed via `--mcp-config`:

```json
{
  "mcpServers": {
    "agentmux": {
      "command": "unused",
      "transport": {
        "type": "sse",
        "url": "http://127.0.0.1:{port}/sse"
      }
    }
  }
}
```

Alternatively, use stdio transport by spawning a thin bridge binary.

#### MCP Tools Provided

| Tool | Description |
|------|-------------|
| `agentmux_list_panes` | List all open panes with their types and titles |
| `agentmux_read_terminal` | Read scrollback buffer from a terminal pane |
| `agentmux_read_editor` | Read content of a code editor pane |
| `agentmux_screenshot` | Take a screenshot of a web preview pane |
| `agentmux_get_selection` | Get the currently selected text in any pane |
| `approve_tool` | Tool approval callback (Phase W-3) |

#### Implementation Location

| File | Purpose |
|------|---------|
| `backend/mcp/mod.rs` | Module root |
| `backend/mcp/server.rs` | TCP listener + JSON-RPC handler |
| `backend/mcp/tools.rs` | Tool implementations |
| `backend/ai/orchestrator.rs` | MCP server lifecycle tied to agent spawn/kill |

### Phase W-6: Enhanced UI Polish

**Goal:** Make the wrapper experience feel native and polished.

#### Cost Display

The status bar gains a cost indicator:

```
[running] claude-sonnet-4-5 | 1,523 tokens | $0.042 | [^C] [reset]
```

Cost comes from the `result` event's `total_cost_usd` field. During streaming, estimate from token counts using known pricing.

#### Thinking/Reasoning Blocks

Claude Code emits `thinking` content blocks (with extended thinking enabled). Map to `ReasoningPart`:

```rust
ClaudeCodeContentBlock::Thinking { thinking, .. } => {
    events.push(AdapterEvent::ReasoningDelta { text: thinking.clone() });
}
```

The view already renders `ReasoningPart` as a collapsible block.

#### Compact Boundary Indicator

When Claude Code compacts its conversation history (`system` event with `subtype: "compact_boundary"`), show a visual separator:

```
─── conversation compacted ───
```

#### Progress Indicators

During multi-turn tool execution, show a breadcrumb of what Claude Code is doing:

```
Reading files... → Analyzing code... → Writing changes...
```

Derived from tool names in the stream.

---

## 5. Configuration

### User Settings

Add to AgentMux settings schema (`schema/settings.json`):

```json
{
  "ai:claudecode:model": {
    "type": "string",
    "description": "Model to use for Claude Code (e.g., claude-sonnet-4-5)",
    "default": ""
  },
  "ai:claudecode:apiKey": {
    "type": "string",
    "description": "Anthropic API key for Claude Code",
    "default": ""
  },
  "ai:claudecode:maxTurns": {
    "type": "integer",
    "description": "Maximum agentic turns before stopping",
    "default": 25
  },
  "ai:claudecode:autoApprove": {
    "type": "array",
    "items": { "type": "string" },
    "description": "Tool patterns to auto-approve (e.g., ['Read', 'Glob', 'Grep'])",
    "default": ["Read", "Glob", "Grep", "Bash(git *)"]
  },
  "ai:claudecode:disallowedTools": {
    "type": "array",
    "items": { "type": "string" },
    "description": "Tools to disable entirely",
    "default": []
  }
}
```

These map to Claude Code CLI flags:
- `ai:claudecode:model` → `ANTHROPIC_MODEL` env var
- `ai:claudecode:apiKey` → `ANTHROPIC_API_KEY` env var
- `ai:claudecode:maxTurns` → `--max-turns` flag
- `ai:claudecode:autoApprove` → `--allowedTools` flag
- `ai:claudecode:disallowedTools` → `--disallowedTools` flag

### Version Pinning

AgentMux should check the installed Claude Code version and warn if it's below the minimum supported version. The wrapper relies on features added in recent versions.

```rust
fn check_claude_code_version(binary: &Path) -> Result<String, Error> {
    let output = Command::new(binary).arg("--version").output()?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Parse and validate against minimum
    Ok(version)
}
```

Minimum supported version: `1.0.0` (stream-json + input-format support).

---

## 6. Known Issues and Mitigations

| Issue | Impact | Mitigation |
|-------|--------|------------|
| Missing `result` event ([#1920](https://github.com/anthropics/claude-code/issues/1920)) | Subprocess hangs after completion | Implement 30s timeout after last `assistant` event. Detect process exit as implicit session end. |
| OOM at 3.3GB+ ([#13126](https://github.com/anthropics/claude-code/issues/13126)) | Subprocess killed by OS | Set `NODE_OPTIONS=--max-old-space-size=4096`. Monitor memory via `/proc/{pid}/status`. Show warning at 2GB. |
| `--input-format stream-json` hangs ([#3187](https://github.com/anthropics/claude-code/issues/3187)) | Second message never processes | Fallback to `--continue` mode: kill subprocess, respawn with `--continue`. |
| Extended thinking breaks streaming | No `stream_event` events emitted | When thinking is enabled, rely on `assistant` complete messages instead of streaming. Show "Thinking..." indicator. |
| `canUseTool` SDK hang ([#4775](https://github.com/anthropics/claude-code/issues/4775)) | Only affects SDK approach | We use CLI `--permission-prompt-tool` instead, not affected. |
| Windows JSON parse errors ([#14442](https://github.com/anthropics/claude-code/issues/14442)) | NDJSON lines corrupted on Windows | Strip BOM, normalize line endings before JSON parse. |

### Defensive Measures

1. **Process watchdog:** If no output for 60s and process is still alive, send health check.
2. **Memory monitor:** Poll `/proc/{pid}/status` every 30s. Warn user at 2GB RSS.
3. **Graceful degradation:** If `stream_event` parsing fails, fall back to `assistant` complete messages only (no streaming, but still functional).
4. **Timeout on result:** Don't wait forever for the `result` event. Process exit is sufficient to end the session.

---

## 7. Testing Strategy

### Unit Tests (Rust)

- Parse all 6 event types from hardcoded JSON fixtures (wire compatibility).
- Round-trip serde for new `ClaudeCodeEvent` variants.
- `adapt_claude_code_event()` produces correct `AdapterEvent` sequences.
- `adapt_claude_code_assistant_message()` reconciliation logic.
- Multi-turn stdin message formatting.

### Integration Tests

- Spawn a mock "Claude Code" binary that emits canned NDJSON events. Verify adapter events arrive at the frontend correctly.
- Tool approval round-trip: mock approval request → UI decision → mock response.
- Session resume: spawn with `--resume`, verify session ID passed correctly.

### Manual Testing Checklist

- [ ] Single-turn: type prompt, see streaming response
- [ ] Multi-turn: send follow-up in same session
- [ ] Tool use: see tool blocks rendered correctly
- [ ] Tool approval: approve and deny destructive operations
- [ ] Interrupt: Ctrl+C stops current generation
- [ ] Reset: start fresh session in same pane
- [ ] Cost: see token count and USD cost in status bar
- [ ] Error: see graceful error display on API failure
- [ ] Resume: restart app, resume previous session
- [ ] Thinking: see collapsible reasoning blocks

---

## 8. Implementation Order

| Phase | Description | Depends On | Est. Lines |
|-------|-------------|------------|------------|
| W-1 | Enhanced NDJSON adapter | Phase A (done) | ~300 Rust |
| W-2 | Multi-turn stdin | W-1 | ~100 Rust |
| W-3 | Tool approval UI | W-1 | ~200 Rust, ~150 TS/SCSS |
| W-4 | Session management | W-2 | ~150 Rust, ~50 TS |
| W-5 | MCP pane awareness | W-1 | ~500 Rust |
| W-6 | UI polish (cost, thinking, compact) | W-1 | ~100 Rust, ~200 TS/SCSS |

**Recommended order:** W-1 → W-2 → W-6 → W-3 → W-4 → W-5

W-1 and W-2 are the foundation. W-6 provides immediate user-visible improvement. W-3 (tool approval) is the most complex and can wait until the basic flow is solid. W-4 (session resume) is a nice-to-have. W-5 (MCP pane awareness) is a separate feature that can be developed in parallel once W-1 is done.

---

## 9. Open Questions

1. **SDK vs CLI:** Should we eventually migrate from CLI subprocess to the TypeScript Agent SDK (`@anthropic-ai/claude-agent-sdk`)? The SDK provides `canUseTool` callbacks and tighter integration, but adds a Node.js dependency. **Decision: Start with CLI, evaluate SDK later.**

2. **API key management:** Should AgentMux manage its own Anthropic API key, or pass through the user's existing Claude Code API key? **Decision: Pass through. Users configure API key in Claude Code's own settings or via AgentMux settings.**

3. **Multiple Claude Code instances:** Should one pane = one Claude Code subprocess, or should we share a subprocess across panes? **Decision: One per pane. Simpler, isolated, matches current architecture.**

4. **Cost limits:** Should AgentMux enforce a per-session or per-day cost limit? **Deferred to Phase D (pricing).**
