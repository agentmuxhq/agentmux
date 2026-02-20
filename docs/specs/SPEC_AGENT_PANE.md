# Agent Pane — Full Specification

> **Version:** 0.30.1 | **Last Updated:** 2026-02-18 | **Status:** Implemented (partial)

---

## 1. Overview

The **Agent Pane** is a first-class widget in AgentMux that displays a live, interactive document representing a Claude Code session. It runs `claude --output-format stream-json` as a child process (via the `cmd` controller), parses the NDJSON output into structured document nodes, and renders them as a scrollable living document with collapsible tool blocks, markdown content, agent-to-agent messages, and user input.

**Widget ID:** `defwidget@agent`
**View type:** `agent`
**Controller:** `cmd`
**Icon:** `sparkles` (color `#cc785c`)

---

## 2. Architecture

```
┌─────────────────────────────────────────────────────┐
│                   AgentMux Window                    │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │              Agent Pane (block)                │  │
│  │                                               │  │
│  │  ┌─ AgentHeader ──────────────────────────┐   │  │
│  │  │ status │ backend │ controls             │   │  │
│  │  └────────────────────────────────────────┘   │  │
│  │                                               │  │
│  │  ┌─ FilterControls ─┐  ┌─ Document ───────┐  │  │
│  │  │ thinking  [x]    │  │ MarkdownBlock    │  │  │
│  │  │ tools ok  [x]    │  │ ToolBlock        │  │  │
│  │  │ tools err [x]    │  │ MarkdownBlock    │  │  │
│  │  │ incoming  [x]    │  │ AgentMessageBlock│  │  │
│  │  │ outgoing  [x]    │  │ UserMessageBlock │  │  │
│  │  └──────────────────┘  │ ToolBlock        │  │  │
│  │                        │ ...              │  │  │
│  │                        └──────────────────┘  │  │
│  │                                               │  │
│  │  ┌─ AgentFooter ─────────────────────────┐   │  │
│  │  │ [textarea input]              [send]   │   │  │
│  │  └────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Data Flow

```
claude --output-format stream-json
    │
    ▼  (stdout → file subject: claude-code.jsonl)
agentmuxsrv-rs (Rust backend)
    │
    ▼  (RxJS file subject subscription)
AgentViewModel.handleTerminalData()
    │
    ▼  (NDJSON line → JSON.parse)
ClaudeCodeStreamParser.parseEvent()
    │
    ▼  (StreamEvent → DocumentNode)
atoms.documentAtom  ← append
    │
    ▼  (filter applied)
filteredDocumentAtom (derived)
    │
    ▼  (React render)
AgentViewInner → renderNode()
```

### User Input (reverse path)

```
AgentFooter textarea
    │
    ▼  sendMessage(text)
AgentViewModel.sendMessage()
    │
    ├─ API mode: ClaudeCodeApiClient.sendMessage()
    │
    └─ Terminal mode: RpcApi.ControllerInputCommand()
           │
           ▼  (base64 encoded text + "\n")
       agentmuxsrv-rs → pty write
           │
           ▼
       claude process stdin
```

---

## 3. Widget Definition

**File:** `pkg/wconfig/defaultconfig/widgets.json`

```json
{
  "defwidget@agent": {
    "display:order": -5,
    "icon": "sparkles",
    "color": "#cc785c",
    "label": "agent",
    "description": "Unified AI agent with streaming output and tool execution",
    "blockdef": {
      "meta": {
        "view": "agent",
        "controller": "cmd",
        "cmd": "claude",
        "cmd:args": ["--output-format", "stream-json"],
        "cmd:interactive": true,
        "cmd:runonstart": true
      }
    }
  }
}
```

| Meta Key           | Value                              | Purpose                                     |
|--------------------|------------------------------------|---------------------------------------------|
| `view`             | `"agent"`                          | Selects `AgentViewModel` from BlockRegistry  |
| `controller`       | `"cmd"`                            | Uses cmd controller (spawns child process)   |
| `cmd`              | `"claude"`                         | Executable to run                            |
| `cmd:args`         | `["--output-format","stream-json"]`| Claude Code outputs NDJSON stream            |
| `cmd:interactive`  | `true`                             | PTY mode (interactive terminal)              |
| `cmd:runonstart`   | `true`                             | Starts automatically when block is created   |

---

## 4. Frontend Components

All files in `frontend/app/view/agent/`.

### 4.1 Registration

**File:** `frontend/app/block/block.tsx:46`

```typescript
BlockRegistry.set("agent", AgentViewModel);
```

### 4.2 ViewModel — `AgentViewModel`

**File:** `agent-model.ts`
**Implements:** `ViewModel` interface

| Property / Method        | Type / Signature                          | Purpose                                      |
|--------------------------|-------------------------------------------|----------------------------------------------|
| `viewType`               | `"agent"`                                 | View type identifier                          |
| `blockId`                | `string`                                  | Unique block identifier                       |
| `atoms`                  | `AgentAtoms`                              | Instance-scoped Jotai atoms                   |
| `parser`                 | `ClaudeCodeStreamParser`                  | NDJSON → DocumentNode converter               |
| `useApiMode`             | `boolean`                                 | API vs terminal mode flag                     |
| `sendMessage(text)`      | `async (string) => void`                  | Send user input (API or terminal)             |
| `exportDocument(format)` | `(string) => void`                        | Export document as markdown/HTML (TODO)        |
| `pauseAgent()`           | `() => void`                              | Pause agent (sets state)                       |
| `resumeAgent()`          | `() => void`                              | Resume agent (sets state)                      |
| `killAgent()`            | `() => void`                              | Send SIGINT to process                         |
| `restartAgent()`         | `() => void`                              | Clear document + force restart via RPC         |
| `dispose()`              | `() => void`                              | Unsubscribe all listeners                      |

**Connection modes:**
1. **Terminal mode** (default): Subscribes to `getFileSubject(blockId, "claude-code.jsonl")` RxJS subject. Receives `{fileop, data64}` messages. Sends input via `RpcApi.ControllerInputCommand`.
2. **API mode** (when connected): Uses `ClaudeCodeApiClient` to stream responses directly. (Currently stubbed — `initializeConnectionMode` checks auth but API key wiring is TODO.)

**Lifecycle:**
- Constructor: creates atoms, subscribes to `controllerstatus` events, initializes parser, checks connection mode
- `connectToTerminal()`: subscribes to file subject for NDJSON output
- `handleTerminalData()`: parses each NDJSON line, appends `DocumentNode[]` to `atoms.documentAtom`
- `dispose()`: unsubscribes file subject, controller status, resets parser

### 4.3 View Component — `AgentViewWrapper` / `AgentViewInner`

**File:** `agent-view.tsx`

`AgentViewWrapper` adapts `ViewComponentProps<AgentViewModel>` to `AgentViewProps`, passing all atoms and callbacks.

`AgentViewInner` renders:
1. **`AgentHeader`** — process status, backend indicator, controls (pause/resume/kill/restart)
2. **Filter toggle button** — shows/hides `FilterControls` sidebar
3. **Document area** — scrollable div, auto-scrolls to bottom on new nodes
4. **Empty state** — "Agent {id} is idle" when no document nodes
5. **Footer** — `ConnectionStatus` (when disconnected) or `AgentFooter` (message input)

**Node rendering** (`renderNode`):
| Node Type        | Component              | Behavior                                      |
|------------------|------------------------|-----------------------------------------------|
| `markdown`       | `MarkdownBlock`        | Renders raw markdown                           |
| `section`        | `<h1>`/`<h2>`/`<h3>`  | Section headings                               |
| `tool`           | `ToolBlock`            | Collapsible, shows params/result/duration      |
| `agent_message`  | `AgentMessageBlock`    | Collapsible, shows from/to/method/direction    |
| `user_message`   | Inline `<pre>`         | Shows user message with 👤 icon               |

### 4.4 Sub-Components

| Component               | File                          | Purpose                                         |
|--------------------------|-------------------------------|------------------------------------------------|
| `AgentHeader`            | `components/AgentHeader.tsx`  | Status icons, backend indicator, process controls|
| `AgentFooter`            | `components/AgentFooter.tsx`  | Minimal textarea input + send                    |
| `MarkdownBlock`          | `components/MarkdownBlock.tsx`| Renders markdown content                         |
| `ToolBlock`              | `components/ToolBlock.tsx`    | Collapsible tool execution display               |
| `AgentMessageBlock`      | `components/AgentMessageBlock.tsx` | Agent-to-agent message display              |
| `ProcessControls`        | `components/ProcessControls.tsx`   | Pause, resume, kill, restart buttons        |
| `FilterControls`         | `components/FilterControls.tsx`    | Filter toggles for document display         |
| `BashOutputViewer`       | `components/BashOutputViewer.tsx`  | Terminal output viewer for Bash results     |
| `DiffViewer`             | `components/DiffViewer.tsx`        | Diff visualization for Edit results         |
| `ConnectionStatus`       | `components/ConnectionStatus.tsx`  | Claude Code auth UI                         |
| `InitializationPrompt`   | `components/InitializationPrompt.tsx` | Init prompt display                      |

---

## 5. State Management

**File:** `state.ts`

All state is **instance-scoped** — each agent widget gets its own atoms via `createAgentAtoms(blockId)`. This prevents state bleeding when multiple agent widgets exist in the same tab.

### 5.1 Atom Set (`AgentAtoms`)

| Atom                  | Type                                 | Default                    | Purpose                        |
|-----------------------|--------------------------------------|----------------------------|--------------------------------|
| `documentAtom`        | `PrimitiveAtom<DocumentNode[]>`      | `[]`                       | The living document            |
| `documentStateAtom`   | `PrimitiveAtom<DocumentState>`       | `{collapsedNodes, filter}` | UI state (collapse, filters)   |
| `streamingStateAtom`  | `PrimitiveAtom<StreamingState>`      | `{active:false, ...}`      | Stream connection state        |
| `processAtom`         | `PrimitiveAtom<AgentProcessState>`   | `{status:"idle", ...}`     | Process lifecycle state        |
| `messageRouterAtom`   | `PrimitiveAtom<MessageRouterState>`  | `{backend:"local", ...}`   | Backend connection mode        |
| `authAtom`            | `PrimitiveAtom<AuthState>`           | `{status:"disconnected"}`  | Claude Code auth state         |
| `userInfoAtom`        | `PrimitiveAtom<UserInfo|null>`       | `null`                     | Authenticated user info        |

### 5.2 Derived Atoms

| Factory Function              | Returns                        | Purpose                              |
|-------------------------------|--------------------------------|--------------------------------------|
| `createFilteredDocumentAtom`  | `Atom<DocumentNode[]>`         | Applies filter to document           |
| `createDocumentStatsAtom`     | `Atom<{totalNodes, ...}>`      | Counts by node type and status       |

### 5.3 Action Atoms

| Factory Function              | Signature                      | Purpose                              |
|-------------------------------|--------------------------------|--------------------------------------|
| `createToggleNodeCollapsed`   | `(nodeId: string) => void`     | Toggle collapse state of a node      |
| `createExpandAllNodes`        | `() => void`                   | Expand all collapsed nodes           |
| `createCollapseAllNodes`      | `() => void`                   | Collapse all nodes                   |
| `createAppendDocumentNode`    | `(node: DocumentNode) => void` | Append node to document              |
| `createUpdateDocumentNode`    | `(node: DocumentNode) => void` | Update node by ID                    |
| `createClearDocument`         | `() => void`                   | Clear document and reset state       |
| `createUpdateFilter`          | `(partial) => void`            | Update filter settings               |

### 5.4 Filter State

| Filter                 | Default  | Controls                                           |
|------------------------|----------|-----------------------------------------------------|
| `showThinking`         | `false`  | Show/hide thinking blocks (hidden by default)        |
| `showSuccessfulTools`  | `true`   | Show/hide successful tool results                    |
| `showFailedTools`      | `true`   | Show/hide failed tool results                        |
| `showIncoming`         | `true`   | Show/hide incoming agent messages                    |
| `showOutgoing`         | `true`   | Show/hide outgoing agent messages                    |

---

## 6. Stream Parsing

**File:** `stream-parser.ts`
**Class:** `ClaudeCodeStreamParser`

Parses NDJSON output from `claude --output-format stream-json`. Each line is a JSON object representing one stream event.

### 6.1 Input → Output Mapping

| Stream Event Type   | Output DocumentNode Type | Collapsed? | Notes                                    |
|---------------------|--------------------------|------------|------------------------------------------|
| `text`              | `MarkdownNode`           | n/a        | Raw markdown content                      |
| `thinking`          | `MarkdownNode`           | n/a        | Has `metadata.thinking = true`            |
| `tool_call`         | `ToolNode`               | No         | Status `"running"`, stored as pending     |
| `tool_result`       | `ToolNode`               | Success=yes, Fail=no | Replaces pending tool call by ID |
| `agent_message`     | `AgentMessageNode`       | Outgoing=yes | Direction inferred from current agent ID |
| `user_message`      | `UserMessageNode`        | No         | Always visible                            |

### 6.2 Tool Summary Generation

Format: `{icon} {tool} {detail} ({duration}s) {statusIcon}`

Examples:
- `📖 Read auth.ts (0.3s) ✓`
- `🔧 Bash git status... (1.2s) ✗`
- `✏️ Edit config.ts (0.1s) ✓`

### 6.3 Pending Tool Tracking

Tool calls and results are linked by ID:
1. `tool_call` event → create `ToolNode` with `status:"running"`, store in `pendingToolCalls` Map
2. `tool_result` event → retrieve params from `pendingToolCalls`, create completed `ToolNode`, remove from pending

---

## 7. Type System

**File:** `types.ts`

### 7.1 DocumentNode Union

```typescript
type DocumentNode = MarkdownNode | SectionNode | ToolNode | AgentMessageNode | UserMessageNode;
```

### 7.2 Key Interfaces

**ToolNode:**
```typescript
interface ToolNode {
    type: "tool";
    id: string;
    tool: "Read" | "Edit" | "Bash" | "Write" | "Grep" | "Glob" | "Task" | "Other";
    params: ToolParams;
    status: "running" | "success" | "failed";
    duration?: number;
    result?: ToolResult;
    collapsed: boolean;
    summary: string;
}
```

**AgentMessageNode:**
```typescript
interface AgentMessageNode {
    type: "agent_message";
    id: string;
    from: string;
    to: string;
    message: string;
    method: "mux" | "ject";      // mux = async mailbox, ject = terminal injection
    direction: "incoming" | "outgoing";
    timestamp: number;
    collapsed: boolean;
    summary: string;
}
```

### 7.3 Icon Mappings

| Category    | Key      | Icon |
|-------------|----------|------|
| Tool        | Read     | 📖   |
| Tool        | Edit     | ✏️   |
| Tool        | Write    | 📝   |
| Tool        | Bash     | 🔧   |
| Tool        | Grep     | 🔍   |
| Tool        | Glob     | 📁   |
| Tool        | Task     | 🛠️   |
| Status      | running  | ⏳   |
| Status      | success  | ✓    |
| Status      | failed   | ✗    |
| Message     | mux      | 📨   |
| Message     | ject     | ⚡   |
| Direction   | incoming | 📥   |
| Direction   | outgoing | 📤   |

---

## 8. Backend Integration — Reactive Messaging

The agent pane integrates with the reactive messaging subsystem for agent-to-agent communication.

### 8.1 Rust Backend Endpoints

**File:** `agentmuxsrv-rs/src/server/reactive.rs` + `agentmuxsrv-rs/src/backend/reactive.rs`

All endpoints are on the **no-auth** path (localhost only):

| Method | Path                         | Handler                       | Purpose                        |
|--------|------------------------------|-------------------------------|--------------------------------|
| POST   | `/wave/reactive/inject`      | `handle_reactive_inject`      | Inject message into agent PTY  |
| GET    | `/wave/reactive/agents`      | `handle_reactive_agents`      | List all registered agents     |
| GET    | `/wave/reactive/agent?id=X`  | `handle_reactive_agent`       | Get single agent details       |
| GET    | `/wave/reactive/audit`       | `handle_reactive_audit`       | Audit log (100-entry ring buf) |
| POST   | `/wave/reactive/register`    | `handle_reactive_register`    | Register agent → block mapping |
| POST   | `/wave/reactive/unregister`  | `handle_reactive_unregister`  | Unregister agent               |
| GET    | `/wave/reactive/poller/stats`| `handle_reactive_poller_stats`| Cross-host poller stats        |
| POST   | `/wave/reactive/poller/config`|`handle_reactive_poller_config`| Configure poller URL/token    |
| GET    | `/wave/reactive/poller/status`|`handle_reactive_poller_status`| Poller running/configured     |

### 8.2 Core Types (Rust)

```rust
struct InjectionRequest {
    target_agent: String,       // Agent ID to inject into
    message: String,            // Message content (max 10,000 bytes)
    source_agent: Option<String>,
    request_id: Option<String>,
    priority: Option<String>,   // "low" | "normal" | "high" | "urgent"
    wait_for_idle: bool,
}

struct AgentRegistration {
    agent_id: String,
    block_id: String,
    tab_id: Option<String>,
    registered_at: u64,
    last_seen: u64,
}
```

### 8.3 Security

| Layer              | Implementation                                         |
|--------------------|---------------------------------------------------------|
| Message sanitize   | Strip ANSI/OSC/CSI sequences, control chars, truncate to 10KB |
| Agent ID validate  | 1-64 chars, alphanumeric + underscore + hyphen only      |
| URL validation     | SSRF protection: HTTP only for localhost, HTTPS elsewhere|
| Rate limiting      | Token bucket: 10 injections/second                       |
| Audit logging      | 100-entry ring buffer with SHA256 message hash           |

### 8.4 Cross-Host Polling (AgentMux Cloud)

The `Poller` continuously polls a remote AgentMux cloud service for pending injections destined for local agents. Config is loaded from `agentmux.json` in the wave data directory.

| Config Key        | Type     | Purpose                  |
|-------------------|----------|--------------------------|
| `agentmux_url`    | `String` | Cloud service base URL   |
| `agentmux_token`  | `String` | Bearer token for auth    |
| `poll_interval_secs` | `u64` | Poll interval (default 30s) |

Flow: `poll /reactive/pending/{agent_id}` → deliver locally → `POST /reactive/ack`

---

## 9. Process Lifecycle

The agent pane manages the Claude Code process through the `cmd` controller.

### 9.1 States

| State     | Meaning                                         |
|-----------|--------------------------------------------------|
| `init`    | Controller created, process not yet started       |
| `running` | Claude process is active and accepting input      |
| `done`    | Process exited (check exit code)                  |

### 9.2 Controller Events

Subscribed via: `waveEventSubscribe({ eventType: "controllerstatus", scope: makeORef("block", blockId) })`

The `BlockControllerRuntimeStatus` provides:
- `shellprocstatus`: `"init"` | `"running"` | `"done"`
- `shellprocexitcode`: number
- `version`: monotonically increasing version for ordering

### 9.3 User Actions

| Action     | Implementation                                       |
|------------|------------------------------------------------------|
| Send input | `ControllerInputCommand({ blockid, inputdata64 })`   |
| Kill       | `ControllerInputCommand({ blockid, signame: "SIGINT" })` |
| Restart    | `ControllerResyncCommand({ tabid, blockid, forcerestart: true })` |
| Pause      | Sets `processAtom.status = "paused"` (UI only)       |
| Resume     | Sets `processAtom.status = "running"` (UI only)      |

---

## 10. Known Gaps and Missing Functionality

### 10.1 WebSocket RPC Commands Not Implemented (Rust Backend)

The following commands are called by the frontend but not handled by the Rust backend's WebSocket RPC engine:

| Command               | Impact                                    | Priority |
|------------------------|-------------------------------------------|----------|
| `setmeta`              | Block metadata updates fail silently       | **High** |
| `getwaveaichat`        | AI chat panel non-functional               | Medium   |
| `getwaveairatelimit`   | Rate limit checks fail                     | Medium   |

Currently implemented WS RPC commands (only 6):
- `getfullconfig`, `routeannounce`, `routeunannounce`
- `eventsub`, `eventunsub`, `eventunsuball`
- `setblocktermsize` + `blockinput` (stubbed as no-ops)

### 10.2 API Mode Not Wired

`AgentViewModel.initializeConnectionMode()` checks for Claude Code auth but the API key wiring is TODO. The API client exists (`api-client.ts`) but is never instantiated with real credentials.

### 10.3 Export Not Implemented

`exportDocument()` logs to console but doesn't produce actual markdown/HTML output.

### 10.4 Pause/Resume Are UI-Only

`pauseAgent()` and `resumeAgent()` only update the Jotai atom — they don't actually pause/resume the underlying process. True pause would require `SIGSTOP`/`SIGCONT` or equivalent.

### 10.5 Agent Registration Flow

The OSC 16162 "E" command (agent environment announcement) should trigger auto-registration via the `/wave/reactive/register` endpoint. The frontend-to-backend wiring for this automatic registration needs verification on the Rust backend.

### 10.6 Stream Parser Limitations

- No handling for `system` event type from Claude Code
- No handling for `result` event type (conversation end with cost data)
- Tool result nodes replace the running node conceptually but are appended as new nodes (duplicate IDs in document array)

---

## 11. File Index

| File                                          | Lines | Purpose                          |
|-----------------------------------------------|-------|----------------------------------|
| `frontend/app/view/agent/agent-model.ts`      | 359   | ViewModel — lifecycle, I/O       |
| `frontend/app/view/agent/agent-view.tsx`       | 225   | React view component             |
| `frontend/app/view/agent/state.ts`             | 339   | Jotai atom factories             |
| `frontend/app/view/agent/types.ts`             | 310   | TypeScript type definitions      |
| `frontend/app/view/agent/stream-parser.ts`     | 308   | NDJSON → DocumentNode parser     |
| `frontend/app/view/agent/api-client.ts`        | —     | Claude Code API client           |
| `frontend/app/view/agent/init-monitor.ts`      | —     | Initialization monitoring        |
| `frontend/app/view/agent/agent-view.scss`      | —     | Styling                          |
| `frontend/app/view/agent/components/*.tsx`      | —     | Sub-components (11 files)        |
| `frontend/app/view/agent/index.ts`             | —     | Barrel exports                   |
| `agentmuxsrv-rs/src/backend/reactive.rs`       | 1443  | Core reactive messaging engine   |
| `agentmuxsrv-rs/src/server/reactive.rs`        | 137   | HTTP endpoint handlers           |
| `agentmuxsrv-rs/src/server/mod.rs`             | ~160  | Route registration               |
| `pkg/wconfig/defaultconfig/widgets.json`        | 40    | Widget definition                |
| `frontend/app/block/block.tsx`                  | —     | BlockRegistry (line 46)          |

---

## 12. Summary

The Agent Pane is a rich, interactive widget that transforms raw Claude Code NDJSON output into a structured living document. It supports:

- **Real-time streaming** of markdown, tool calls, and agent messages
- **Instance-scoped state** preventing cross-widget contamination
- **Collapsible nodes** with filter controls for managing information density
- **Agent-to-agent messaging** via mux (async mailbox) and ject (terminal injection)
- **Process lifecycle management** (start, kill, restart)
- **Cross-host delivery** via AgentMux cloud polling

Key areas needing work: missing WS RPC commands (`setmeta`, `getwaveaichat`, `getwaveairatelimit`), API mode credential wiring, and document export functionality.
