# Agent Widget Translation Layer Analysis

**Date:** 2026-02-16
**Status:** Analysis Complete
**Purpose:** Document the translation layer between Claude Code CLI and AgentMux UI

---

## Overview

The agent widget acts as a **translation layer** between:
1. **Claude Code CLI** - Command-line tool with NDJSON streaming output
2. **AgentMux UI** - React-based interactive document viewer
3. **Backend Routing** - Process lifecycle and I/O management

---

## Architecture Layers

```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: UI Presentation (React/Jotai)                │
│  - agent-view.tsx: Document rendering                  │
│  - Components: MarkdownBlock, ToolBlock, etc.          │
│  - State: Instance-scoped Jotai atoms                  │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│  Layer 2: ViewModel Translation (TypeScript)           │
│  - agent-model.ts: Lifecycle management                │
│  - stream-parser.ts: NDJSON → DocumentNode[]           │
│  - api-client.ts: Cloud API integration                │
│  - state.ts: Atom factories, filters, actions          │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│  Layer 3: I/O Relay (Frontend ↔ Backend)              │
│  - getFileSubject(): Subscribe to claude-code.jsonl    │
│  - RpcApi.ControllerInputCommand(): Send user input    │
│  - waveEventSubscribe(): Process status updates        │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│  Layer 4: Process Management (Go Backend)              │
│  - blockcontroller.go: Controller lifecycle            │
│  - shellcontroller.go: Command execution               │
│  - shellexec.go: PTY/process management                │
└──────────────────┬──────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────┐
│  Layer 5: Claude Code CLI Process                      │
│  - Command: claude --output-format stream-json         │
│  - Output: NDJSON stream to claude-code.jsonl          │
│  - Input: STDIN for user messages                      │
└─────────────────────────────────────────────────────────┘
```

---

## Layer 1: UI Presentation

**Files:**
- `frontend/app/view/agent/agent-view.tsx`
- `frontend/app/view/agent/components/*.tsx`
- `frontend/app/view/agent/agent-view.scss`

**Responsibilities:**
1. Render document nodes as interactive markdown
2. Handle user interactions (expand/collapse, filters, send messages)
3. Display connection status and process controls
4. Auto-scroll to latest content
5. Show typing indicators and loading states

**Key Components:**
```tsx
AgentViewInner
├── AgentHeader           // Stats, title
├── ConnectionStatus      // Auth status, disconnect button
├── FilterControls        // Show/hide tool blocks, etc.
├── ProcessControls       // Pause, kill, restart
├── Document Rendering
│   ├── MarkdownBlock     // Prose paragraphs
│   ├── ToolBlock         // Collapsible tool execution
│   ├── AgentMessageBlock // Agent-to-agent messages
│   ├── BashOutputViewer  // Terminal output
│   └── DiffViewer        // Code diffs
└── AgentFooter           // Input area
```

**State Management:**
- Uses **instance-scoped Jotai atoms** (not global)
- Each widget instance has its own atom set
- Allows multiple agent widgets to coexist without state collision

---

## Layer 2: ViewModel Translation

**Files:**
- `frontend/app/view/agent/agent-model.ts` - Core translation logic
- `frontend/app/view/agent/stream-parser.ts` - NDJSON parsing
- `frontend/app/view/agent/state.ts` - Atom factories
- `frontend/app/view/agent/api-client.ts` - Cloud API (future)

### AgentViewModel Initialization

```typescript
constructor(blockId: string, nodeModel: BlockNodeModel) {
    // 1. Create instance-scoped atoms
    this.atoms = createAgentAtoms(blockId);

    // 2. Subscribe to process status
    this.procStatusUnsub = waveEventSubscribe({
        eventType: "controllerstatus",
        scope: WOS.makeORef("block", blockId),
        handler: (event) => this.updateShellProcStatus(event.data)
    });

    // 3. Initialize stream parser
    this.parser = new ClaudeCodeStreamParser();

    // 4. Determine connection mode
    await this.initializeConnectionMode();
}
```

### Connection Mode Selection

```typescript
async initializeConnectionMode() {
    const authStatus = await getApi().getClaudeCodeAuth();

    if (authStatus.connected) {
        // API Mode: Use claude.ai cloud
        this.useApiMode = true;
        // TODO: Initialize API client with real key
    } else {
        // Local Mode: Use claude CLI process
        this.useApiMode = false;
        this.connectToTerminal();
    }
}
```

### Terminal Connection (Local Mode)

```typescript
connectToTerminal() {
    // Subscribe to claude-code.jsonl file updates
    this.fileSubjectRef = getFileSubject(this.blockId, "claude-code.jsonl");
    this.fileSubjectSub = this.fileSubjectRef.subscribe((msg) => {
        this.handleTerminalData(msg);
    });
}

async handleTerminalData(msg: any) {
    if (msg.fileop === "append" && msg.data64) {
        const text = new TextDecoder().decode(base64ToArray(msg.data64));

        // Parse NDJSON stream
        const lines = text.split('\n').filter(line => line.trim());
        for (const line of lines) {
            const event = JSON.parse(line);
            const nodes = await this.parser.parseEvent(event);

            // Append to THIS instance's document
            const currentDoc = globalStore.get(this.atoms.documentAtom);
            globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
        }
    }
}
```

**Key Translation:** NDJSON events → DocumentNode[] → React components

---

## Layer 3: I/O Relay

### Output Stream (Backend → Frontend)

```typescript
// Backend writes to: ~/.agentmux/blocks/{blockId}/claude-code.jsonl
// Frontend subscribes via:
getFileSubject(blockId, "claude-code.jsonl").subscribe(handleData)
```

**File Subject Protocol:**
- `fileop: "truncate"` - File cleared, reset parser
- `fileop: "append"` - New data, parse and append
- `data64` - Base64-encoded content

### Input Stream (Frontend → Backend)

```typescript
async sendMessage(text: string) {
    const b64data = stringToBase64(text + "\n");
    await RpcApi.ControllerInputCommand(TabRpcClient, {
        blockid: this.blockId,
        inputdata64: b64data
    });
}
```

**Input Routing:**
- Frontend → RpcApi.ControllerInputCommand()
- Backend → shellcontroller writes to process STDIN
- Claude Code → reads from STDIN, processes, writes to STDOUT

### Process Status Events

```typescript
waveEventSubscribe({
    eventType: "controllerstatus",
    scope: WOS.makeORef("block", blockId),
    handler: (event) => {
        const status = event.data as BlockControllerRuntimeStatus;
        // status.shellprocstatus: "init" | "running" | "done"
        // status.shellprocexitcode: number
    }
});
```

---

## Layer 4: Process Management (Go Backend)

### Widget Configuration

**File:** `pkg/wconfig/defaultconfig/widgets.json`

```json
{
    "defwidget@agent": {
        "icon": "sparkles",
        "label": "agent",
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

**Metadata Keys:**
- `view: "agent"` - Frontend view type
- `controller: "cmd"` - Backend controller type (non-shell command)
- `cmd: "claude"` - Command to execute
- `cmd:args` - Command arguments array
- `cmd:interactive: true` - Allocate PTY for interactive I/O
- `cmd:runonstart: true` - Auto-start process when block opens

### Controller Lifecycle

**File:** `pkg/blockcontroller/blockcontroller.go`

```go
func StartBlockController(ctx context.Context, tabId string, blockId string) error {
    // 1. Get block metadata
    blockData := wstore.DBMustGet[*waveobj.Block](ctx, blockId)
    controllerName := blockData.Meta.GetString("controller", "")

    // 2. Create controller instance
    if controllerName == "cmd" || controllerName == "shell" {
        controller = MakeShellController(tabId, blockId, controllerName)
    }

    // 3. Start process
    err := controller.Start(ctx, blockData.Meta, rtOpts, force)
}
```

### Command Construction

**File:** `pkg/blockcontroller/shellcontroller.go`

```go
func (sc *ShellController) makeCmdStr(blockMeta MetaMapType) (string, *CommandOptsType) {
    // 1. Get base command
    cmdStr := blockMeta.GetString("cmd", "")  // "claude"

    // 2. Get arguments if cmd:shell is false
    useShell := blockMeta.GetBool("cmd:shell", true)
    if !useShell {
        cmdArgs := blockMeta.GetStringList("cmd:args")
        // ["--output-format", "stream-json"]

        // 3. Shell-quote and append
        for _, arg := range cmdArgs {
            cmdStr = cmdStr + " " + ShellQuote(arg)
        }
    }

    // Result: "claude '--output-format' 'stream-json'"
}
```

### Process Execution

**File:** `pkg/shellexec/shellexec.go`

```go
func RunCommand(cmdStr string, opts CommandOptsType) (*ShellProc, error) {
    // 1. Create PTY
    ptyFile, tty := pty.Start(cmd)

    // 2. Start process
    cmd.Start()

    // 3. Relay I/O
    go io.Copy(outputFile, ptyFile)  // Process output → claude-code.jsonl
    go io.Copy(ptyFile, inputReader) // User input → Process stdin

    // 4. Monitor exit
    go cmd.Wait()
}
```

---

## Layer 5: Claude Code CLI

### Command Executed

```bash
claude --output-format stream-json
```

### Current Arguments
- `--output-format stream-json` - NDJSON streaming output

### Missing Arguments (from spec)
- `--agent-id ${blockId}` - Unique agent identifier
- `--skip-permissions` - Auto-approve all tool executions ⚠️

### Output Format

**File:** `~/.agentmux/blocks/{blockId}/claude-code.jsonl`

```json
{"type":"stream_event","event":{"type":"message_start","message":{"role":"assistant",...}}}
{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}}
{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Let"}}}
{"type":"stream_event","event":{"type":"tool_use_block","id":"toolu_123","name":"bash","input":{"command":"ls"}}}}
{"type":"stream_event","event":{"type":"tool_result_block","tool_use_id":"toolu_123","content":"file1.txt\nfile2.txt"}}
```

### Stream Parser Translation

**File:** `frontend/app/view/agent/stream-parser.ts`

```typescript
async parseEvent(event: any): Promise<DocumentNode[]> {
    switch (event.type) {
        case "message_start":
            return [{
                type: "markdown",
                content: "## New Message\n",
                timestamp: Date.now()
            }];

        case "content_block_delta":
            // Accumulate text deltas into markdown paragraphs

        case "tool_use_block":
            return [{
                type: "tool",
                toolName: event.name,
                toolInput: event.input,
                collapsed: false
            }];

        case "tool_result_block":
            // Append result to existing tool block
    }
}
```

**Translation:** Stream events → Structured document → React components

---

## Current State vs. Spec

### ✅ Implemented

1. **Dual Mode Architecture**
   - Local mode (claude CLI streaming)
   - API mode (claude.ai cloud) - skeleton in place

2. **Stream Parsing**
   - NDJSON → DocumentNode[] translation
   - Markdown, tool blocks, diffs, bash output

3. **Instance Scoping**
   - Per-widget atom sets
   - Multiple agent widgets can coexist

4. **UI Components**
   - Living document interface
   - Collapsible tool blocks
   - Connection status (Phase 5)

5. **Process Lifecycle**
   - Auto-start on block open
   - Kill, restart, pause controls
   - Exit code tracking

### ⚠️ Missing / Incomplete

1. **Permission Handling**
   - ❌ `--skip-permissions` not in cmd:args
   - ❌ No UI state for "waiting for permission approval"
   - ❌ No permission prompt → auto-approve flow

2. **Agent ID**
   - ❌ `--agent-id ${blockId}` not in cmd:args
   - Spec shows this should be passed to Claude Code

3. **API Mode**
   - ⚠️ Skeleton exists but not functional
   - ⚠️ No real API key integration
   - ⚠️ Cloud routing not implemented

4. **State Presentation**
   - ❌ No translation of Claude Code permission states
   - ❌ UI doesn't show "claude is waiting for permission"
   - ❌ No visual distinction between states:
     - Initializing
     - Waiting for user input
     - Waiting for permission approval
     - Executing tool
     - Streaming response

---

## Gap Analysis: Permission State Translation

### Problem

When Claude Code runs WITHOUT `--skip-permissions`, it will:
1. Show a tool to execute
2. **Pause and wait** for user approval
3. User must press Y/N at terminal

**Current AgentMux behavior:**
- Streams output until pause
- UI shows the tool block
- **No indication that approval is needed**
- **No way to approve from UI**
- User must manually type "y" in input box (if that even works)

### What's Missing

**Backend:**
```go
// widgets.json should have:
"cmd:args": [
    "--output-format", "stream-json",
    "--agent-id", "${blockId}",
    "--skip-permissions"  // ⚠️ MISSING
]
```

**Frontend State Machine:**
```typescript
// agent-model.ts should track:
type ClaudeCodeState =
    | "initializing"      // Process starting
    | "ready"             // Waiting for user input
    | "waiting_approval"  // ⚠️ Tool waiting for Y/N
    | "executing_tool"    // Tool running
    | "streaming"         // Typing response
    | "error"             // Failed
    | "exited";           // Process done

// UI should show:
if (state === "waiting_approval") {
    <PermissionPrompt
        tool={currentTool}
        onApprove={() => sendInput("y\n")}
        onReject={() => sendInput("n\n")}
    />
}
```

**Stream Parser:**
```typescript
// stream-parser.ts should detect:
parseEvent(event) {
    if (event.type === "permission_required") {
        // ⚠️ Does this event exist?
        return {
            type: "permission_prompt",
            tool: event.tool,
            requiresApproval: true
        };
    }
}
```

### Recommended Fix

**Option A: Skip Permissions (Easier)**
```json
// widgets.json
"cmd:args": [
    "--output-format", "stream-json",
    "--skip-permissions"  // Auto-approve all tools
]
```

**Pros:**
- Simple 1-line fix
- No UI changes needed
- Works immediately

**Cons:**
- No user control
- Tools execute without confirmation
- Security concern for destructive operations

**Option B: Build Permission UI (Proper)**
1. Add permission prompt component
2. Detect when Claude Code is waiting
3. Send "y\n" or "n\n" via ControllerInputCommand
4. Show pending state in UI

**Pros:**
- User has control
- Safer for destructive tools
- Better UX transparency

**Cons:**
- Requires UI work
- Needs state machine
- Parser must detect permission events

---

## I/O Relay Specs

### File Subject Protocol

**Backend → Frontend (Output)**

```typescript
interface FileMessage {
    fileop: "truncate" | "append";
    data64?: string;  // Base64-encoded content
}

// Example:
{
    fileop: "append",
    data64: "eyJ0eXBlIjoic3RyZWFtX2V2ZW50Ii..."
}
```

**Subscription:**
```typescript
const subject = getFileSubject(blockId, "claude-code.jsonl");
subject.subscribe({
    next: (msg: FileMessage) => {
        if (msg.fileop === "truncate") {
            clearDocument();
        } else if (msg.fileop === "append") {
            const content = base64Decode(msg.data64);
            parseAndAppend(content);
        }
    }
});
```

### Controller Input Protocol

**Frontend → Backend (Input)**

```typescript
interface ControllerInputRequest {
    blockid: string;
    inputdata64?: string;  // Base64-encoded text
    signame?: string;      // SIGINT, SIGTERM, etc.
}

// Send user message:
await RpcApi.ControllerInputCommand(TabRpcClient, {
    blockid: "abc123",
    inputdata64: stringToBase64("Please review this code\n")
});

// Send interrupt:
await RpcApi.ControllerInputCommand(TabRpcClient, {
    blockid: "abc123",
    signame: "SIGINT"
});
```

### Controller Status Protocol

**Backend → Frontend (Lifecycle)**

```typescript
interface BlockControllerRuntimeStatus {
    shellprocstatus: "init" | "running" | "done";
    shellprocexitcode: number;
    shellprocconnname?: string;
}

// Subscribe to status:
waveEventSubscribe({
    eventType: "controllerstatus",
    scope: WOS.makeORef("block", blockId),
    handler: (event: WaveEvent) => {
        const status = event.data as BlockControllerRuntimeStatus;

        if (status.shellprocstatus === "running") {
            showRunning();
        } else if (status.shellprocstatus === "done") {
            showExited(status.shellprocexitcode);
        }
    }
});
```

---

## Summary

### What Exists ✅

1. **Complete I/O relay** between Claude Code process and React UI
2. **Stream parser** that translates NDJSON → DocumentNode[]
3. **Instance-scoped state** allowing multiple agent widgets
4. **Connection mode selection** (local vs cloud)
5. **Process lifecycle management** (start, stop, restart, kill)
6. **File subject** streaming for real-time updates

### What's Missing ⚠️

1. **`--skip-permissions`** flag in cmd:args (or permission approval UI)
2. **`--agent-id`** flag in cmd:args
3. **State machine** for permission approval states
4. **UI presentation** of "waiting for approval" state
5. **API mode** implementation (cloud routing)

### Next Steps 🚀

1. **Add `--skip-permissions` to widgets.json** (immediate fix)
2. **Add `--agent-id ${blockId}` to widgets.json** (per spec)
3. **Document state machine** for permission flows
4. **Implement permission prompt UI** (if removing skip-permissions)
5. **Complete API mode** for cloud integration

---

## References

- **Spec:** `docs/SPEC_UNIFIED_AGENT_WIDGET.md`
- **Implementation:** `frontend/app/view/agent/`
- **Backend:** `pkg/blockcontroller/shellcontroller.go`
- **Config:** `pkg/wconfig/defaultconfig/widgets.json`
