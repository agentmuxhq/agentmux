# Specification: Unified Agent Widget

**Date**: 2026-02-13
**Status**: Draft
**Replaces**: AI Widget (`defwidget@ai`) + Claude Code Widget (`defwidget@claudecode`)

---

## Overview

Consolidate the existing "ai" and "claude code" widgets into a single unified **"agent"** widget with a **multi-pane architecture**:

- **One Widget Per Agent**: Each agent instance runs in its own separate pane
- **Multi-Pane Workspace**: Users can open multiple agent widgets simultaneously (claude-1, reviewer-agent, test-agent, etc.)
- **Backend-Routed Messages**: Agents communicate via the backend; messages appear in the recipient's pane
- **Living Document UI**: Each pane shows an interactive markdown document of that agent's activity
- **Deep Exploration**: Expandable sections to drill into tool executions, diffs, and incoming messages
- **Streaming Intermediary**: Sophisticated layer that converts Claude Code's NDJSON stream into structured, interactive markdown

---

## Goals

1. **Multi-Pane Architecture**: Each agent runs in its own widget pane - users can open multiple agents side-by-side
2. **Per-Agent Monitoring**: Each pane shows one agent's complete activity (tool executions, incoming messages, output)
3. **Backend-Routed Communication**: Messages sent between agents appear in the recipient's pane automatically
4. **Interactive Documentation**: Live markdown document that the agent writes/updates as it works
5. **Deep Observability**: Expandable sections to drill into any level of detail (full diffs, complete tool output, message content)
6. **Reduce Complexity**: Replace two separate widget types with one sophisticated monitoring interface

---

## Current State Analysis

### AI Widget (`defwidget@ai`, view: `waveai`)

**Strengths:**
- Clean chat interface with message bubbles
- Markdown rendering with syntax highlighting
- Code block navigation (previous/next buttons)
- Preset system for quick prompts
- Smooth auto-scrolling to latest message
- Message history with sliding window
- @ai-sdk/react integration for streaming

**Weaknesses:**
- No visibility into tool execution
- Limited control over running processes
- No diff rendering for file changes
- Missing terminal command output display

**Key Files:**
- `frontend/app/aipanel/aipanel.tsx`
- Uses `useChat` hook from @ai-sdk/react
- Jotai atoms: `selectedPresetAtom`, `chatMessagesAtom`

### Claude Code Widget (`defwidget@claudecode`, view: `claudecode`)

**Strengths:**
- Full tool execution transparency
- Collapsible tool blocks (Bash, Edit, Read, etc.)
- Beautiful diff rendering for file edits
- Process management (kill, restart)
- Metadata tracking (exit codes, timing)
- Terminal-native feel
- NDJSON streaming parser

**Weaknesses:**
- Less conversational, more technical
- No markdown chat bubbles
- No preset system
- Harder for non-technical users

**Key Files:**
- `frontend/app/view/claudecode/claudecode-view.tsx`
- Custom NDJSON parser
- Tool block components (ToolBash, ToolEdit, ToolDiff)

---

## Unified Agent Widget Design

### Widget Definition

```json
"defwidget@agent": {
    "display:order": -2,
    "icon": "sparkles",
    "color": "#cc785c",
    "label": "agent",
    "description": "AI agent with streaming output and tool execution",
    "blockdef": {
        "meta": {
            "view": "agent",
            "controller": "cmd",
            "cmd": "claude",
            "cmd:args": ["--output-format", "stream-json", "--agent-id", "${agentId}"],
            "cmd:interactive": true,
            "cmd:runonstart": true,
            "agent:id": "${blockId}"  // Each widget instance gets unique agent ID
        }
    }
}
```

**Multi-Pane Architecture**:
- Each widget instance represents **one agent**
- Agent ID is derived from the block ID (automatically unique)
- Users can create multiple agent widgets: "claude-1", "reviewer", "tester", etc.
- Each agent's output streams to its own pane
- Messages between agents are routed via backend

**Example Workspace**:
```
┌───────────────────┬───────────────────┬───────────────────┐
│  Agent: claude-1  │  Agent: reviewer  │  Agent: tester    │
├───────────────────┼───────────────────┼───────────────────┤
│ # Fixing auth     │ # Code Review     │ # Test Runner     │
│                   │                   │                   │
│ Reading auth.ts   │ 📥 From claude-1  │ Idle...           │
│ ...               │ "Review fix?"     │                   │
│                   │                   │                   │
│ 📤 To reviewer    │ 📤 To claude-1    │                   │
│ "Review fix?"     │ "LGTM"            │                   │
│                   │                   │                   │
│ 📥 From reviewer  │                   │ ⚡ From claude-1  │
│ "LGTM"            │                   │ Run tests         │
│                   │                   │                   │
│ 📤 To tester      │                   │ 🔧 Bash: npm test │
│ Run tests (ject)  │                   │ ✓ 5/5 passed      │
│                   │                   │                   │
│ 👤 User:          │                   │ 📤 To claude-1    │
│ "Also check .env" │                   │ "Tests pass ✓"    │
│                   │                   │                   │
├───────────────────┼───────────────────┼───────────────────┤
│ [Send to agent...│ [Send to agent... │ [Send to agent... │
└───────────────────┴───────────────────┴───────────────────┘
                              ▲
                              │
                    ┌─────────┴─────────┐
                    │  Backend Routing  │
                    │                   │
                    │  Local: agentmux  │
                    │  Cloud: agentbus  │
                    └───────────────────┘
```

---

## UI/UX Design

### Core Concept: Living Markdown Document

**NOT a chat interface.** Instead, think of a **live technical document** that Claude Code is writing/updating in real-time, similar to:
- Jupyter notebook (interactive cells)
- Observable notebook (reactive markdown)
- GitHub README with embedded demos
- Technical spec document with expandable implementation details

### Single Agent Pane Layout

**Example**: Agent ID = "claude-1" (developer agent)

```
┌─────────────────────────────────────────────┐
│  Agent: claude-1               [⚙️ Kill]    │  ← Pane header with agent ID
├─────────────────────────────────────────────┤
│                                             │
│  # Debugging Login Authentication           │  ← Agent's task (self-written)
│                                             │
│  ## Analysis                                │  ← Section
│                                             │
│  I'll investigate the authentication flow   │  ← Agent's markdown output
│  in `auth.ts` to identify the login issue.  │
│                                             │
│  <details>                                  │  ← Tool execution (by this agent)
│    <summary>📖 Read auth.ts (0.3s) ✓</summary>│
│    ```typescript                            │
│    export function login(user, pass) {      │
│      const hash = hashPassword(pass);       │
│      if (user.password == hash) { // BUG!   │
│    ```                                      │
│  </details>                                 │
│                                             │
│  ## Root Cause Found                        │
│                                             │
│  Password comparison on line 42 uses `==`   │
│  instead of bcrypt. I'll propose a fix.     │
│                                             │
│  <details>                                  │  ← Outgoing message (sent by this agent)
│    <summary>📤 To reviewer (mux) ✓</summary>│
│    **Sent**: 14:32:15                       │
│    **Message**: Can you review my proposed  │
│    fix before I apply it?                   │
│                                             │
│    ```diff                                  │
│    - if (user.password == hash) {           │
│    + if (await bcrypt.compare(pass, hash)) {│
│    ```                                      │
│  </details>                                 │
│                                             │
│  <details>                                  │  ← INCOMING message (from another agent)
│    <summary>📥 From reviewer (mux) ✓</summary>│
│    **Received**: 14:32:47                   │
│    **Message**: LGTM. Also add error        │
│    handling for bcrypt failures.            │
│  </details>                                 │
│                                             │
│  ## Applying Fix                            │
│                                             │
│  Implementing the fix with error handling   │
│  as suggested by reviewer.                  │
│                                             │
│  <details>                                  │  ← Tool execution
│    <summary>✏️ Edit auth.ts (1.2s) ✓</summary>│
│    ```diff                                  │
│    @@ auth.ts:42 @@                         │
│    - if (user.password == hash) {           │
│    + try {                                  │
│    +   if (await bcrypt.compare(...)) {     │
│    ```                                      │
│  </details>                                 │
│                                             │
│  <details>                                  │  ← Outgoing message
│    <summary>📤 To tester (ject) ✓</summary> │
│    **Sent**: 14:33:15                       │
│    **Command**: npm test auth.test.ts       │
│    **Type**: Terminal injection             │
│  </details>                                 │
│                                             │
│  ## Summary                                 │
│                                             │
│  ✅ Fix applied with error handling         │
│  ✅ Waiting for test results from tester    │
│                                             │
├─────────────────────────────────────────────┤
│  [Send message to claude-1...     ] [Send] │  ← User input
│  [Export] [Clear]                           │
└─────────────────────────────────────────────┘
```

**Meanwhile, in "reviewer" agent's pane**:

```
┌─────────────────────────────────────────────┐
│  Agent: reviewer                            │
├─────────────────────────────────────────────┤
│                                             │
│  # Code Review Queue                        │
│                                             │
│  Waiting for review requests...             │
│                                             │
│  <details>                                  │  ← INCOMING message (appears here!)
│    <summary>📥 From claude-1 (mux) ✓</summary>│
│    **Received**: 14:32:15                   │
│    **Message**: Can you review my proposed  │
│    fix before I apply it?                   │
│                                             │
│    ```diff                                  │
│    - if (user.password == hash) {           │
│    + if (await bcrypt.compare(pass, hash)) {│
│    ```                                      │
│  </details>                                 │
│                                             │
│  ## Review Result                           │
│                                             │
│  The fix looks good, but needs error        │
│  handling for bcrypt failures.              │
│                                             │
│  <details>                                  │  ← Outgoing reply
│    <summary>📤 To claude-1 (mux) ✓</summary>│
│    **Sent**: 14:32:47                       │
│    **Message**: LGTM. Also add error        │
│    handling for bcrypt failures.            │
│  </details>                                 │
│                                             │
└─────────────────────────────────────────────┘
```

**And in "tester" agent's pane**:

```
┌─────────────────────────────────────────────┐
│  Agent: tester                              │
├─────────────────────────────────────────────┤
│                                             │
│  # Test Runner                              │
│                                             │
│  Idle, waiting for test commands...         │
│                                             │
│  <details>                                  │  ← INCOMING ject (terminal injection!)
│    <summary>⚡ From claude-1 (ject) ✓</summary>│
│    **Received**: 14:33:15                   │
│    **Injected command**: npm test auth.test.ts│
│    **Execution starting...**                │
│  </details>                                 │
│                                             │
│  <details>                                  │  ← Tool execution (triggered by ject)
│    <summary>🔧 Bash: npm test (3.5s) ✓</summary>│
│    ```                                      │
│    ✓ auth.test.ts (5/5 passed)             │
│    ✓ login with valid credentials          │
│    ✓ login with invalid credentials        │
│    ```                                      │
│  </details>                                 │
│                                             │
│  <details>                                  │  ← Outgoing result
│    <summary>📤 To claude-1 (mux) ✓</summary>│
│    **Sent**: 14:33:19                       │
│    **Message**: All auth tests passing ✓    │
│  </details>                                 │
│                                             │
└─────────────────────────────────────────────┘
```

### Component Hierarchy

```tsx
<AgentView>
  <AgentHeader>
    <AgentStatus agentId={agentId} status="running" />
    <AgentControls /> {/* pause, restart, kill */}
  </AgentHeader>

  <AgentDocument>
    {/* Streaming intermediary converts NDJSON → DocumentNode[] */}
    <StreamingMarkdownRenderer
      stream={agentOutputStream}
      parser={claudeCodeStreamParser}
    >
      {documentNodes.map(node => (
        node.type === 'markdown' ? (
          <MarkdownBlock content={node.content} />
        ) : node.type === 'tool' ? (
          <InteractiveToolBlock
            tool={node.tool}
            params={node.params}
            result={node.result}
            status={node.status}
            collapsed={node.collapsed}
          >
            {node.tool === 'Edit' ? (
              <DiffViewer diff={node.result.diff} interactive={true} />
            ) : node.tool === 'Bash' ? (
              <BashOutputViewer
                output={node.result.stdout}
                stderr={node.result.stderr}
                exitCode={node.result.exitCode}
              />
            ) : node.tool === 'Read' ? (
              <FilePreview path={node.params.file_path} />
            ) : node.tool === 'AgentMessage' ? (
              <AgentMessageBlock
                from={node.params.from}
                to={node.params.to}
                message={node.params.message}
                type={node.params.type} {/* 'mux' | 'ject' */}
              />
            ) : null}
          </InteractiveToolBlock>
        ) : node.type === 'section' ? (
          <MarkdownSection
            level={node.level}
            title={node.title}
            collapsible={true}
          />
        ) : null
      ))}
    </StreamingMarkdownRenderer>
  </AgentDocument>

  <AgentFooter>
    <UserInput
      placeholder="Send message to agent..."
      onSend={(message) => sendUserMessage(agentId, message)}
    />
    <ExportButton format="markdown" />
    <ClearButton />
  </AgentFooter>
</AgentView>
```

**User Input**: Each agent pane has an input box where users can asynchronously send messages to that agent.

---

## Feature Consolidation

### From AI Widget → Agent Widget

| Feature | Status | Notes |
|---------|--------|-------|
| Chat bubbles | ❌ Remove | Replaced with markdown paragraphs |
| Markdown rendering | ✅ Keep | Enhanced with interactive elements |
| Code block navigation | ✅ Keep | Prev/next buttons for code blocks |
| Preset system | ⚠️ Remove | Not needed for monitoring |
| Smooth scrolling | ✅ Keep | Auto-scroll to latest activity |
| Message history | ✅ Keep | Full document history |
| @ai-sdk/react | ❌ Replace | Use custom NDJSON streaming parser |

### From Claude Code Widget → Agent Widget

| Feature | Status | Notes |
|---------|--------|-------|
| Tool execution blocks | ✅ Keep | Collapsible by default |
| Diff rendering | ✅ Keep | For Edit tool visualization |
| Bash output display | ✅ Keep | With ANSI color support |
| Process management | ✅ Keep | Kill, restart, pause controls |
| Metadata tracking | ✅ Keep | Exit codes, timing, status |
| NDJSON parser | ✅ Keep | More flexible than @ai-sdk/react |
| Terminal-native feel | ⚠️ Soften | Make more accessible to non-devs |

### New Features for Agent Widget

| Feature | Priority | Description |
|---------|----------|-------------|
| Living markdown document | P0 | Interactive document that updates as agents work |
| Streaming intermediary | P0 | NDJSON → DocumentNode conversion layer |
| Agent communication viz | P0 | Show mux/ject messages between agents |
| Smart section inference | P1 | Auto-detect sections (Analysis, Findings, Solution) |
| Collapsible tool groups | P1 | Group related tools (Read → Edit → Test) |
| Filter by agent | P1 | Show only specific agent's activity |
| Show/hide thinking | P1 | Toggle visibility of thinking blocks |
| Tool execution timeline | P2 | Visual timeline of all tool calls |
| Copy tool results | P2 | Copy button for tool outputs |
| Export full document | P2 | Save complete markdown with all expansions |
| Agent activity dashboard | P3 | Summary of all active agents + stats |

---

## Streaming Intermediary Layer

**Core Innovation**: A sophisticated parser that converts Claude Code's raw NDJSON stream into a structured markdown document.

### Architecture

```
┌─────────────────────┐
│   Claude Code CLI   │
│  (--output-format   │
│   stream-json)      │
└──────────┬──────────┘
           │ NDJSON Stream
           ▼
┌─────────────────────────────────────────────┐
│        Streaming Intermediary Layer         │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │  1. Stream Parser                   │   │
│  │     - Read NDJSON line by line      │   │
│  │     - Identify event types          │   │
│  │     - Buffer incomplete events      │   │
│  └──────────┬──────────────────────────┘   │
│             │ Parsed Events                │
│             ▼                               │
│  ┌─────────────────────────────────────┐   │
│  │  2. Document Generator              │   │
│  │     - Convert events → DocumentNode │   │
│  │     - Infer structure (sections)    │   │
│  │     - Group related tools           │   │
│  │     - Format markdown               │   │
│  └──────────┬──────────────────────────┘   │
│             │ Document Nodes               │
│             ▼                               │
│  ┌─────────────────────────────────────┐   │
│  │  3. State Manager                   │   │
│  │     - Track collapsed/expanded      │   │
│  │     - Manage scroll position        │   │
│  │     - Handle user interactions      │   │
│  └──────────┬──────────────────────────┘   │
│             │ Stateful Document            │
└─────────────┼─────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│     Interactive Markdown Renderer           │
│  (React components with expand/collapse)    │
└─────────────────────────────────────────────┘
```

### Stream Parser

**Input**: Claude Code NDJSON stream

```jsonlines
{"type":"text","text":"I'll debug the login issue."}
{"type":"thinking","text":"Need to read auth.ts first..."}
{"type":"tool_call","tool":"Read","id":"call_1","params":{"file_path":"auth.ts"}}
{"type":"tool_result","tool":"Read","id":"call_1","status":"success","duration":0.3,"result":"..."}
{"type":"text","text":"Found the issue on line 42."}
{"type":"tool_call","tool":"Edit","id":"call_2","params":{"file_path":"auth.ts","old_string":"...","new_string":"..."}}
{"type":"agent_message","from":"claude-1","to":"reviewer","message":"Ready for review","method":"mux"}
```

**Output**: Typed events

```typescript
type StreamEvent =
  | { type: 'text'; content: string }
  | { type: 'thinking'; content: string }
  | { type: 'tool_call'; tool: string; id: string; params: any }
  | { type: 'tool_result'; tool: string; id: string; status: string; result: any }
  | { type: 'agent_message'; from: string; to: string; message: string; method: 'mux' | 'ject' }
  | { type: 'user_message'; message: string }; // User input to agent
```

### Document Generator

**Input**: Stream events
**Output**: Structured markdown document nodes

```typescript
type DocumentNode =
  | MarkdownNode
  | SectionNode
  | ToolNode
  | AgentMessageNode
  | UserMessageNode;

interface MarkdownNode {
  type: 'markdown';
  id: string;
  content: string; // Raw markdown text
  metadata?: { thinking?: boolean };
}

interface SectionNode {
  type: 'section';
  id: string;
  level: 1 | 2 | 3; // H1, H2, H3
  title: string;
  collapsible: boolean;
  collapsed: boolean;
}

interface ToolNode {
  type: 'tool';
  id: string;
  tool: 'Read' | 'Edit' | 'Bash' | 'Write' | 'Grep' | 'Glob';
  params: Record<string, any>;
  status: 'running' | 'success' | 'failed';
  duration?: number;
  result?: any;
  collapsed: boolean;
  summary: string; // e.g., "📖 Read auth.ts (0.3s) ✓"
}

interface AgentMessageNode {
  type: 'agent_message';
  id: string;
  from: string; // Agent ID
  to: string; // Agent ID (this agent)
  message: string;
  method: 'mux' | 'ject';
  direction: 'incoming' | 'outgoing';
  timestamp: number;
  collapsed: boolean;
  summary: string; // e.g., "📨 claude-1 → reviewer (mux)" or "📥 From claude-1 (mux)"
}

interface UserMessageNode {
  type: 'user_message';
  id: string;
  message: string;
  timestamp: number;
  collapsed: boolean;
  summary: string; // "👤 User Message"
}
```

**Generation Rules**:

1. **Text blocks** → MarkdownNode
   - Render as paragraphs
   - Support inline code, bold, italic, links

2. **Tool calls** → ToolNode (collapsed by default)
   - Icon based on tool type (📖 Read, ✏️ Edit, 🔧 Bash)
   - Status indicator (⏳ running, ✓ success, ✗ failed)
   - Duration in summary
   - Expandable to show params + results

3. **Agent messages** → AgentMessageNode (collapsed by default)
   - Icon: 📨 for mux, ⚡ for ject
   - Show from/to agent names
   - Expandable to show full message content

4. **Section inference**:
   - Agent says "Let me analyze..." → H2: Analysis
   - Agent says "I found the issue..." → H2: Findings
   - Agent says "Here's the fix..." → H2: Solution
   - Use NLP patterns to detect section boundaries

5. **Grouping**:
   - Multiple related tool calls → Group under one `<details>`
   - Example: Read file → Edit file → Run tests = "Apply fix" group

### State Manager

Tracks UI state for the document:

```typescript
interface DocumentState {
  nodes: DocumentNode[];
  collapsedNodes: Set<string>; // Node IDs that are collapsed
  scrollPosition: number;
  selectedNode: string | null; // For keyboard navigation
  filter: {
    showThinking: boolean;
    showSuccessfulTools: boolean;
    agentFilter: string | null; // Show only specific agent
  };
}

// User interactions
function toggleNode(nodeId: string): void;
function expandAll(): void;
function collapseAll(): void;
function filterByAgent(agentId: string): void;
function exportMarkdown(): string;
```

## Technical Architecture

### State Management (Jotai Atoms)

**Important**: Each agent widget instance has its own state (scoped to that block).

```typescript
// Per-widget state (scoped to block)
export const agentIdAtom = atom<string>(''); // This agent's ID
export const agentDocumentAtom = atom<DocumentNode[]>([]); // This agent's document

export const documentStateAtom = atom({
  collapsedNodes: new Set<string>(),
  scrollPosition: 0,
  selectedNode: null as string | null,
  filter: {
    showThinking: false,          // Hide thinking by default
    showSuccessfulTools: true,    // Show successful tools
    showFailedTools: true,        // Always show failures
    showIncoming: true,           // Show incoming messages
    showOutgoing: true,           // Show outgoing messages
  },
});

// Streaming state (this agent's output stream)
export const streamingStateAtom = atom({
  active: boolean;
  bufferSize: number; // Number of events buffered
  lastEventTime: number;
});

// Process control (this agent's process)
export const agentProcessAtom = atom<{
  pid?: number;
  status: 'idle' | 'running' | 'paused' | 'failed';
  canRestart: boolean;
  canKill: boolean;
}>({
  pid: undefined,
  status: 'idle',
  canRestart: true,
  canKill: false,
});

// Message routing (backend connection)
export const messageRouterAtom = atom<{
  backend: 'local' | 'cloud'; // agentmux backend vs agentbus
  connected: boolean;
  endpoint: string;
}>({
  backend: 'local',
  connected: false,
  endpoint: '',
});
```

**Backend Integration**:

```typescript
// Subscribe to incoming messages for this agent
async function subscribeToMessages(agentId: string): Promise<void> {
  const router = useAtomValue(messageRouterAtom);

  if (router.backend === 'local') {
    // Local: WebSocket to agentmux backend
    const ws = new WebSocket(`${router.endpoint}/agent/${agentId}/messages`);
    ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      handleIncomingMessage(message);
    };
  } else {
    // Cloud: Subscribe via agentbus
    const agentbus = getAgentBusClient();
    await agentbus.subscribe(agentId, (message) => {
      handleIncomingMessage(message);
    });
  }
}

// Handle incoming message and add to document
function handleIncomingMessage(message: AgentMessage): void {
  const node: AgentMessageNode = {
    type: 'agent_message',
    id: `msg_${Date.now()}`,
    from: message.from,
    to: message.to,
    message: message.content,
    method: message.method,
    timestamp: message.timestamp,
    collapsed: false, // Incoming messages expanded by default
    direction: 'incoming',
    summary: `📥 From ${message.from} (${message.method})`,
  };

  // Append to this agent's document
  const currentDoc = agentDocumentAtom.get();
  agentDocumentAtom.set([...currentDoc, node]);
}
```

### Backend Communication

**Message Routing**:
- **Local deployment**: Messages route through agentmux backend (localhost)
- **Cloud deployment**: Messages route through agentbus (already integrated)

**Stream Format:** NDJSON (Claude CLI `--output-format stream-json`)

Extended to include agent-to-agent communication events:

```jsonlines
{"type":"text","text":"I'll help debug the authentication issue."}
{"type":"thinking","text":"First, I need to read the auth file to understand the current implementation."}
{"type":"tool_call","tool":"Read","id":"call_1","params":{"file_path":"/app/auth.ts"}}
{"type":"tool_result","tool":"Read","id":"call_1","status":"success","duration":0.3,"result":"export function login..."}
{"type":"text","text":"Found the issue on line 42. I'll fix it now."}
{"type":"tool_call","tool":"Edit","id":"call_2","params":{"file_path":"/app/auth.ts","old_string":"==","new_string":"bcrypt.compare"}}
{"type":"tool_result","tool":"Edit","id":"call_2","status":"success","duration":1.2,"result":{"linesChanged":1}}
{"type":"agent_message","from":"claude-1","to":"reviewer-agent","message":"Ready for code review","method":"mux"}
{"type":"agent_message","from":"reviewer-agent","to":"claude-1","message":"LGTM, tests pass","method":"mux"}
{"type":"tool_call","tool":"Bash","id":"call_3","params":{"command":"git commit -m 'fix: use bcrypt for password comparison'"}}
```

**Parser Implementation**:

```typescript
class ClaudeCodeStreamParser {
  private buffer: string = '';
  private nodeIdCounter: number = 0;

  async *parse(stream: ReadableStream<Uint8Array>): AsyncGenerator<DocumentNode> {
    const reader = stream.getReader();
    const decoder = new TextDecoder();

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      this.buffer += decoder.decode(value, { stream: true });
      const lines = this.buffer.split('\n');
      this.buffer = lines.pop() || ''; // Keep incomplete line

      for (const line of lines) {
        if (!line.trim()) continue;

        try {
          const event = JSON.parse(line);
          const node = this.eventToNode(event);
          if (node) yield node;
        } catch (err) {
          console.error('Failed to parse NDJSON line:', line, err);
        }
      }
    }
  }

  private eventToNode(event: any): DocumentNode | null {
    switch (event.type) {
      case 'text':
        return {
          type: 'markdown',
          id: `node_${this.nodeIdCounter++}`,
          content: event.text,
        };

      case 'thinking':
        return {
          type: 'markdown',
          id: `node_${this.nodeIdCounter++}`,
          content: event.text,
          metadata: { thinking: true },
        };

      case 'tool_call':
        // Store tool call, wait for result
        return {
          type: 'tool',
          id: event.id,
          tool: event.tool,
          params: event.params,
          status: 'running',
          collapsed: true,
          summary: this.generateToolSummary(event.tool, event.params, 'running'),
        };

      case 'tool_result':
        // Update existing tool node with result
        return {
          type: 'tool',
          id: event.id,
          tool: event.tool,
          params: {}, // Already stored from tool_call
          status: event.status,
          duration: event.duration,
          result: event.result,
          collapsed: event.status === 'success', // Collapse successes, expand failures
          summary: this.generateToolSummary(event.tool, {}, event.status, event.duration),
        };

      case 'agent_message':
        return {
          type: 'agent_message',
          id: `msg_${this.nodeIdCounter++}`,
          from: event.from,
          to: event.to,
          message: event.message,
          method: event.method,
          timestamp: Date.now(),
          collapsed: true,
          summary: `${this.getAgentIcon(event.method)} ${event.from} → ${event.to}`,
        };

      default:
        console.warn('Unknown event type:', event.type);
        return null;
    }
  }

  private generateToolSummary(
    tool: string,
    params: any,
    status: string,
    duration?: number
  ): string {
    const icon = this.getToolIcon(tool);
    const statusIcon = status === 'running' ? '⏳' : status === 'success' ? '✓' : '✗';
    const durationStr = duration ? ` (${duration.toFixed(1)}s)` : '';

    const detail = params.file_path || params.command || params.pattern || '';
    return `${icon} ${tool} ${detail}${durationStr} ${statusIcon}`.trim();
  }

  private getToolIcon(tool: string): string {
    const icons: Record<string, string> = {
      Read: '📖',
      Edit: '✏️',
      Write: '📝',
      Bash: '🔧',
      Grep: '🔍',
      Glob: '📁',
    };
    return icons[tool] || '🛠️';
  }

  private getAgentIcon(method: string): string {
    return method === 'mux' ? '📨' : '⚡';
  }
}
```

### Multi-Agent Communication Monitoring

When agents communicate (via `mux` or `ject`), these events appear in the document:

```markdown
## Agent Communication

<details>
  <summary>📨 claude-1 → reviewer-agent (mux) - "Ready for code review"</summary>

  **From**: claude-1
  **To**: reviewer-agent
  **Method**: mux (async mailbox)
  **Time**: 2026-02-13 14:32:15

  > Ready for code review

  **Context**:
  - 3 files changed
  - 2 tool calls executed
  - 1 test passed

  [View full thread] [Reply]
</details>

<details>
  <summary>📨 reviewer-agent → claude-1 (mux) - "LGTM, tests pass"</summary>

  **From**: reviewer-agent
  **To**: claude-1
  **Method**: mux (async mailbox)
  **Time**: 2026-02-13 14:32:47

  > LGTM, tests pass

  [View full thread]
</details>
```

---

## Implementation Plan

### Phase 1: Streaming Intermediary (Week 1)

**Goal:** Build the core NDJSON → DocumentNode conversion layer

- [ ] Create `frontend/app/view/agent/stream-parser.ts`
  - [ ] NDJSON stream parser
  - [ ] Event type definitions
  - [ ] DocumentNode type definitions
  - [ ] Event → Node conversion logic
- [ ] Create `frontend/app/view/agent/document-generator.ts`
  - [ ] Section inference (Analysis, Findings, Solution)
  - [ ] Tool grouping logic
  - [ ] Summary generation for tool/agent message nodes
- [ ] Create `frontend/app/view/agent/state.ts`
  - [ ] Jotai atoms for document state
  - [ ] Collapsed/expanded tracking
  - [ ] Filter state management
- [ ] Unit tests for parser + generator

### Phase 2: Interactive Markdown UI (Week 2)

**Goal:** Build React components for rendering the living document

- [ ] Create `frontend/app/view/agent/agent-view.tsx`
  - [ ] Main container component
  - [ ] Streaming markdown renderer
  - [ ] Auto-scroll to latest activity
- [ ] Create `frontend/app/view/agent/components/`
  - [ ] `MarkdownBlock.tsx` - Render markdown paragraphs
  - [ ] `ToolBlock.tsx` - Collapsible tool execution details
  - [ ] `AgentMessageBlock.tsx` - Agent-to-agent communication
  - [ ] `DiffViewer.tsx` - Port from claudecode-view
  - [ ] `BashOutputViewer.tsx` - Port from claudecode-view
- [ ] Add collapsible `<details>` wrapper component
- [ ] CSS styling for document layout

### Phase 3: Multi-Agent Features (Week 2-3)

**Goal:** Add agent monitoring and communication visualization

- [ ] Agent message node rendering
  - [ ] Mux/ject icon differentiation
  - [ ] From/to agent display
  - [ ] Thread view for conversations
- [ ] Agent activity tracking
  - [ ] Track active agents in `activeAgentsAtom`
  - [ ] Show agent status (active, idle, error)
  - [ ] Agent filter dropdown
- [ ] Advanced filtering
  - [ ] Show/hide thinking blocks
  - [ ] Show/hide successful tools
  - [ ] Filter by agent ID
- [ ] Process management
  - [ ] Pause/resume streaming
  - [ ] Kill agent process
  - [ ] Restart agent

### Phase 4: Polish & Advanced Features (Week 3-4)

**Goal:** Add sophisticated monitoring features

- [ ] Smart section inference
  - [ ] NLP patterns for section detection
  - [ ] Auto-generate H2 headings
  - [ ] Collapsible sections
- [ ] Tool grouping
  - [ ] Detect related tool sequences (Read → Edit → Bash)
  - [ ] Group under single expandable block
  - [ ] Summary: "Applied fix (3 tools, 5.2s)"
- [ ] Export functionality
  - [ ] Export as markdown (with all expansions)
  - [ ] Export as HTML
  - [ ] Copy to clipboard
- [ ] Code block navigation (from AI widget)
- [ ] Keyboard navigation (vim-style j/k)

### Phase 5: Migration & Cleanup (Week 4)

**Goal:** Replace old widgets, ship v1.0

- [ ] Update `widgets.json`
  - [ ] Remove `defwidget@ai`
  - [ ] Remove `defwidget@claudecode`
  - [ ] Add `defwidget@agent`
- [ ] Register agent view in `frontend/app/view/view.tsx`
- [ ] Workspace migration logic
  - [ ] Auto-replace old widgets with agent widget
  - [ ] Preserve position/size
- [ ] Remove old code
  - [ ] Delete `frontend/app/aipanel/`
  - [ ] Delete `frontend/app/view/claudecode/`
- [ ] Documentation
  - [ ] Update user guide
  - [ ] Add agent monitoring tutorial
  - [ ] Record demo video
- [ ] Testing
  - [ ] E2E tests for streaming parser
  - [ ] Visual regression tests
  - [ ] Multi-agent communication tests

---

## Migration Strategy

### User Impact

**Before:** Users have two separate AI widgets
- `ai` widget: Chat-style, simple, no tool visibility
- `claude code` widget: Terminal-style, power users, full tool visibility

**After:** Users have one unified agent widget
- `agent` widget: Chat + tools, best of both worlds

### Migration Path

1. **Automatic migration** (v1.0 → v1.1):
   - On first launch after upgrade, scan user's workspace for `ai` or `claudecode` widgets
   - Replace with `agent` widget in same position
   - Preserve widget size, pinned state, etc.

2. **Backward compatibility**:
   - Keep old widget IDs registered for 1 release cycle
   - Show deprecation warning if user tries to create old widgets
   - Final removal in v1.2

3. **Documentation**:
   - Update docs to only mention `agent` widget
   - Add "What happened to AI/Claude Code widgets?" FAQ section

---

## Multi-Agent Communication

Each agent pane shows **incoming and outgoing messages** from that agent's perspective.

### Communication Types

**Mux (📨)**: Asynchronous mailbox delivery
- Sender: Message appears as "📤 To <recipient> (mux)"
- Recipient: Message appears as "📥 From <sender> (mux)"
- Routed through backend (agentmux local or agentbus cloud)

**Ject (⚡)**: Direct terminal injection
- Sender: Appears as "📤 To <recipient> (ject)" with command
- Recipient: Appears as "⚡ From <sender> (ject)" with auto-executed command
- Triggers immediate tool execution in recipient's terminal

### Example: Three-Agent Workflow

**Agent Pane 1: "claude-1" (developer)**

```markdown
# Fixing Authentication Bug

## Analysis

I'll read the auth code first.

<details>
  <summary>📖 Read auth.ts (0.3s) ✓</summary>
  ...
</details>

Found bug on line 42. Requesting review before fixing.

<details>
  <summary>📤 To reviewer (mux) ✓</summary>
  **Sent**: 14:32:15
  **Message**: Can you review this fix?

  ```diff
  - if (user.password == hash) {
  + if (await bcrypt.compare(pass, hash)) {
  ```
</details>

<details>
  <summary>📥 From reviewer (mux)</summary>
  **Received**: 14:32:47
  **Message**: LGTM. Add error handling too.
</details>

## Applying Fix

Implementing with error handling as suggested.

<details>
  <summary>✏️ Edit auth.ts (1.2s) ✓</summary>
  ...
</details>

<details>
  <summary>📤 To tester (ject) ✓</summary>
  **Sent**: 14:33:15
  **Command**: npm test auth.test.ts
  **Type**: Terminal injection
</details>

<details>
  <summary>📥 From tester (mux)</summary>
  **Received**: 14:33:19
  **Message**: All tests passing ✓
</details>

## Complete

✅ Fix applied with error handling
✅ Tests passing
```

**Agent Pane 2: "reviewer" (code reviewer)**

```markdown
# Code Review Queue

Waiting for review requests...

<details>
  <summary>📥 From claude-1 (mux)</summary>
  **Received**: 14:32:15
  **Message**: Can you review this fix?

  ```diff
  - if (user.password == hash) {
  + if (await bcrypt.compare(pass, hash)) {
  ```
</details>

## Review

Fix looks good, but needs error handling for bcrypt failures.

<details>
  <summary>📤 To claude-1 (mux) ✓</summary>
  **Sent**: 14:32:47
  **Message**: LGTM. Add error handling too.
</details>
```

**Agent Pane 3: "tester" (test runner)**

```markdown
# Test Runner

Idle, waiting for test requests...

<details>
  <summary>⚡ From claude-1 (ject)</summary>
  **Received**: 14:33:15
  **Injected Command**: npm test auth.test.ts
  **Auto-executing...**
</details>

<details>
  <summary>🔧 Bash: npm test auth.test.ts (3.5s) ✓</summary>
  ```
  ✓ auth.test.ts (5/5 passed)
  ✓ login with valid credentials
  ✓ login with invalid credentials
  ```
</details>

<details>
  <summary>📤 To claude-1 (mux) ✓</summary>
  **Sent**: 14:33:19
  **Message**: All tests passing ✓
</details>
```

### Message Routing Architecture

```
┌───────────────────┐
│ Agent: claude-1   │
│ Widget Instance   │
│                   │
│ 📤 send("reviewer",│
│    "Review fix")  │
└─────────┬─────────┘
          │
          ▼
┌─────────────────────────┐
│  Backend Routing Layer  │
│                         │
│  Local:  agentmux       │
│  Cloud:  agentbus       │
└─────────┬───────────────┘
          │
          ▼
┌───────────────────┐
│ Agent: reviewer   │
│ Widget Instance   │
│                   │
│ 📥 receive from   │
│    claude-1       │
└───────────────────┘
```

**Key Points**:
- Each widget connects to backend with its agent ID
- Backend routes messages to correct recipient widget
- Local: WebSocket to agentmux backend
- Cloud: Agentbus pubsub (already integrated)
- Widgets can be on same machine or different machines (cloud mode)

### User Input to Agents

Each agent pane has an **input box** at the bottom for users to send asynchronous messages to that agent.

**Example**: User sends message to "claude-1" agent

```markdown
# Agent: claude-1

... agent is working on task ...

<details>
  <summary>📖 Read config.ts (0.2s) ✓</summary>
  ...
</details>

<details>
  <summary>👤 User Message</summary>
  **Received**: 14:45:22
  **Message**: Also check the environment variables in .env file
</details>

## Checking Environment

Good point! Let me check the .env file as well.

<details>
  <summary>📖 Read .env (0.1s) ✓</summary>
  ...
</details>
```

**User Input Component**:

```tsx
function UserInput({ agentId, onSend }: UserInputProps) {
  const [message, setMessage] = useState('');

  const handleSend = async () => {
    if (!message.trim()) return;

    // Send user message to this agent via backend
    await sendUserMessage(agentId, message);

    // Append to agent's document immediately
    const node: UserMessageNode = {
      type: 'user_message',
      id: `user_${Date.now()}`,
      message: message,
      timestamp: Date.now(),
      collapsed: false,
      summary: '👤 User Message',
    };

    appendToDocument(node);
    setMessage('');
  };

  return (
    <div className="user-input">
      <input
        type="text"
        placeholder="Send message to agent..."
        value={message}
        onChange={(e) => setMessage(e.target.value)}
        onKeyPress={(e) => e.key === 'Enter' && handleSend()}
      />
      <button onClick={handleSend}>Send</button>
    </div>
  );
}
```

**Backend Handling**:

```typescript
// When user sends message to agent
async function sendUserMessage(agentId: string, message: string): Promise<void> {
  const router = getMessageRouter();

  if (router.backend === 'local') {
    // Send via agentmux backend WebSocket
    ws.send(JSON.stringify({
      type: 'user_message',
      to: agentId,
      message: message,
    }));
  } else {
    // Send via agentbus
    await agentbus.publish(agentId, {
      type: 'user_message',
      message: message,
    });
  }

  // Backend injects message into agent's Claude Code stdin
  // Agent receives it in its stream and can respond
}
```

## Open Questions

1. **Default collapsed state for tool blocks?**
   - Option A: All tools collapsed by default
   - Option B: Failed tools expanded, successful collapsed
   - Option C: User preference (saved in settings)
   - **Recommendation:** Option B - show problems, hide successes

2. **How to handle very long tool outputs (e.g., `Read` of 5000-line file)?**
   - Option A: Truncate with "Show more" button
   - Option B: Virtual scrolling for tool results
   - Option C: Lazy load on expand
   - **Recommendation:** Option C - only load when user expands

3. **Section inference: Use NLP or simple keyword matching?**
   - Option A: Simple regex patterns ("Let me analyze" → H2: Analysis)
   - Option B: Use LLM to generate section titles
   - Option C: No auto-sections, require agent to output explicit markdown headers
   - **Recommendation:** Start with Option A, add Option C as fallback

4. **How to display deeply nested agent communication threads?**
   - Option A: Flat list with indentation
   - Option B: Threaded view (like email/Slack)
   - Option C: Graph visualization (nodes = agents, edges = messages)
   - **Recommendation:** Option B for familiarity, Option C as advanced view

5. **Should thinking blocks be visible by default?**
   - Option A: Hide by default (reduce noise)
   - Option B: Show by default (full transparency)
   - Option C: Show only for failed operations
   - **Recommendation:** Option A - advanced users can toggle on

6. **Tool grouping: Automatic or manual?**
   - Option A: Fully automatic based on heuristics
   - Option B: Agent explicitly marks groups in stream
   - Option C: User can manually group/ungroup in UI
   - **Recommendation:** Option B for accuracy, Option A as fallback

7. **Real-time vs buffered updates?**
   - Option A: Update DOM immediately as each event arrives
   - Option B: Buffer events and batch update every 100ms
   - **Recommendation:** Option B for performance with high-frequency streams

---

## Success Metrics

### Functional Requirements
- [ ] Stream parser converts NDJSON → DocumentNode without errors
- [ ] Document updates in real-time as agent streams output
- [ ] All tool types render correctly (Read, Edit, Bash, Write, Grep, Glob)
- [ ] Agent messages (mux/ject) display from/to/method clearly
- [ ] Tool blocks are collapsible and expand on click
- [ ] Failed tools auto-expand, successful tools auto-collapse
- [ ] File edits show diffs with syntax highlighting
- [ ] Bash commands show stdout/stderr with ANSI colors
- [ ] Filter by agent works correctly
- [ ] Show/hide thinking toggles visibility
- [ ] Export to markdown preserves structure

### Non-Functional Requirements
- [ ] Widget renders initial state in < 100ms
- [ ] Streaming adds nodes with < 50ms latency
- [ ] Smooth scrolling with 1000+ nodes
- [ ] Memory usage < 200MB for large documents
- [ ] No memory leaks during long-running sessions

### Code Quality
- [ ] Single codebase (aipanel + claudecode removed)
- [ ] 80%+ test coverage for stream parser
- [ ] TypeScript strict mode, no `any` types
- [ ] Accessible (keyboard nav, ARIA labels, screen reader support)
- [ ] Dark mode support

---

## Visual Design Guidelines

### Document-First Aesthetic

The agent widget should feel like a **living technical document**, not a chat app.

**Inspiration**:
- Jupyter notebook (code + output cells)
- Notion (interactive blocks)
- GitHub README (markdown with embeds)
- Observable (reactive notebook)

**NOT like**:
- Slack, Discord (chat bubbles)
- ChatGPT (conversational UI)
- Traditional terminals (plain text)

### Typography

```css
/* Document headers */
h1 { font-size: 24px; font-weight: 700; margin: 24px 0 16px; }
h2 { font-size: 20px; font-weight: 600; margin: 20px 0 12px; }
h3 { font-size: 16px; font-weight: 600; margin: 16px 0 8px; }

/* Body text */
p { font-size: 14px; line-height: 1.6; margin: 8px 0; }

/* Code */
code { font-family: 'JetBrains Mono', monospace; font-size: 13px; }
pre { background: var(--code-bg); padding: 12px; border-radius: 6px; }
```

### Color Palette

```css
:root {
  /* Tool icons */
  --tool-read: #3b82f6;    /* Blue */
  --tool-edit: #f59e0b;    /* Orange */
  --tool-bash: #10b981;    /* Green */
  --tool-write: #8b5cf6;   /* Purple */

  /* Status indicators */
  --status-running: #f59e0b;  /* Orange */
  --status-success: #10b981;  /* Green */
  --status-failed: #ef4444;   /* Red */

  /* Agent communication */
  --agent-mux: #3b82f6;    /* Blue */
  --agent-ject: #eab308;   /* Yellow */

  /* Document background */
  --doc-bg: #ffffff;
  --doc-bg-dark: #1e1e1e;
}
```

### Interactive Elements

**Collapsible Details Block**:
```html
<details class="tool-block" data-status="success">
  <summary class="tool-summary">
    <span class="tool-icon">📖</span>
    <span class="tool-name">Read</span>
    <span class="tool-target">auth.ts</span>
    <span class="tool-duration">0.3s</span>
    <span class="tool-status">✓</span>
  </summary>
  <div class="tool-content">
    <!-- Full tool output -->
  </div>
</details>
```

**CSS**:
```css
.tool-block {
  border-left: 3px solid var(--tool-read);
  margin: 8px 0;
  padding: 0;
  background: rgba(59, 130, 246, 0.05);
  border-radius: 4px;
}

.tool-block[data-status="failed"] {
  border-left-color: var(--status-failed);
  background: rgba(239, 68, 68, 0.05);
}

.tool-summary {
  padding: 8px 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: monospace;
  font-size: 13px;
}

.tool-summary:hover {
  background: rgba(0, 0, 0, 0.05);
}

.tool-content {
  padding: 12px;
  border-top: 1px solid rgba(0, 0, 0, 0.1);
}
```

### Layout

```
┌────────────────────────────────────────────────┐
│  Agent Widget                                  │
├────────────────────────────────────────────────┤
│                                                │
│  [Standard markdown paragraph margins]         │
│  [No chat bubble borders or backgrounds]       │
│  [Full width content, not centered bubbles]    │
│                                                │
│  Tool blocks have:                             │
│    - Left border accent (colored by tool type) │
│    - Subtle background tint                    │
│    - Collapsed by default                      │
│    - Smooth expand/collapse animation          │
│                                                │
│  Code blocks have:                             │
│    - Syntax highlighting                       │
│    - Copy button (top right)                   │
│    - Line numbers (for large blocks)           │
│                                                │
└────────────────────────────────────────────────┘
```

### Animations

```css
/* Smooth expand/collapse */
details[open] .tool-content {
  animation: slideDown 200ms ease-out;
}

@keyframes slideDown {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}

/* New node appears */
.document-node.new {
  animation: fadeIn 300ms ease-out;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* Streaming indicator */
.streaming-indicator {
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

## Complete Architecture Overview

### Multi-Pane Agent System

```
┌─────────────────────────────────────────────────────────────────┐
│                        AgentMux Workspace                       │
├─────────────────┬─────────────────┬─────────────────────────────┤
│                 │                 │                             │
│ AGENT WIDGET 1  │ AGENT WIDGET 2  │  AGENT WIDGET 3             │
│ ID: claude-1    │ ID: reviewer    │  ID: tester                 │
│                 │                 │                             │
│ ┌─────────────┐ │ ┌─────────────┐ │  ┌─────────────┐            │
│ │ Agent Output│ │ │ Agent Output│ │  │ Agent Output│            │
│ │ Stream      │ │ │ Stream      │ │  │ Stream      │            │
│ │ (NDJSON →   │ │ │ (NDJSON →   │ │  │ (NDJSON →   │            │
│ │  Markdown)  │ │ │  Markdown)  │ │  │  Markdown)  │            │
│ │             │ │ │             │ │  │             │            │
│ │ - Text      │ │ │ - Text      │ │  │ - Text      │            │
│ │ - Tools     │ │ │ - Tools     │ │  │ - Tools     │            │
│ │ - 📥 From B │ │ │ - 📥 From A │ │  │ - ⚡ From A  │            │
│ │ - 📤 To B   │ │ │ - 📤 To A   │ │  │ - 📤 To A   │            │
│ │ - 👤 User   │ │ │             │ │  │             │            │
│ └─────────────┘ │ └─────────────┘ │  └─────────────┘            │
│                 │                 │                             │
│ [User Input]    │ [User Input]    │  [User Input]               │
│ [Send]          │ [Send]          │  [Send]                     │
│                 │                 │                             │
└────────┬────────┴────────┬────────┴────────┬────────────────────┘
         │                 │                 │
         │  WebSocket/     │                 │
         │  AgentBus       │                 │
         └─────────────────┼─────────────────┘
                           │
                           ▼
         ┌─────────────────────────────────────┐
         │      Backend Message Router         │
         │                                     │
         │  ┌───────────────────────────────┐  │
         │  │  Local Mode                   │  │
         │  │  - agentmux backend           │  │
         │  │  - WebSocket per agent        │  │
         │  │  - Process management         │  │
         │  └───────────────────────────────┘  │
         │                                     │
         │  ┌───────────────────────────────┐  │
         │  │  Cloud Mode                   │  │
         │  │  - agentbus (integrated)      │  │
         │  │  - Cross-machine routing      │  │
         │  │  - Pubsub architecture        │  │
         │  └───────────────────────────────┘  │
         │                                     │
         │  Message Routing:                   │
         │  - Agent A → Agent B (mux/ject)     │
         │  - User → Agent (user_message)      │
         │  - Backend → Agent (stream events)  │
         │                                     │
         └─────────────────────────────────────┘
```

### Message Flow Examples

**1. User sends message to agent**:
```
User types in Agent Widget 1 input box
  ↓
Frontend calls sendUserMessage("claude-1", "Check .env file")
  ↓
Backend routes to claude-1's process stdin
  ↓
Claude Code receives user message in stream
  ↓
Agent responds with markdown/tools
  ↓
Response streams to Agent Widget 1
  ↓
User sees agent's response in document
```

**2. Agent sends mux to another agent**:
```
Agent Widget 1 (claude-1) executes tool "SendMessage"
  ↓
NDJSON: {"type":"agent_message","from":"claude-1","to":"reviewer","method":"mux",...}
  ↓
Agent Widget 1 shows: 📤 To reviewer (mux)
  ↓
Backend routes message to "reviewer" agent
  ↓
Agent Widget 2 (reviewer) receives via WebSocket/AgentBus
  ↓
Agent Widget 2 shows: 📥 From claude-1 (mux)
  ↓
Reviewer agent processes message and can respond
```

**3. Agent sends ject (terminal injection)**:
```
Agent Widget 1 (claude-1) sends ject to tester
  ↓
NDJSON: {"type":"agent_message","from":"claude-1","to":"tester","method":"ject","command":"npm test"}
  ↓
Agent Widget 1 shows: 📤 To tester (ject)
  ↓
Backend injects command into tester's stdin
  ↓
Agent Widget 3 (tester) shows: ⚡ From claude-1 (ject)
  ↓
Tester agent executes command immediately
  ↓
Tool execution appears in Agent Widget 3
  ↓
Tester can send results back via mux
```

### Key Design Principles

1. **One Widget = One Agent**: Each widget instance monitors exactly one agent
2. **Bidirectional Communication**: Agents ↔ Agents, Users → Agents, Backend → Agents
3. **Backend Routing**: All messages route through backend (local or cloud)
4. **Live Document**: Each widget renders a living markdown document of its agent's activity
5. **Async Input**: Users can send messages to any agent at any time via input box
6. **Expandable Details**: All tool executions, messages, and events are collapsible
7. **Multi-Deployment**: Works in local mode (agentmux) and cloud mode (agentbus)

## References

- Existing spec: `docs/SPEC_TABBAR_ENHANCEMENTS.md`
- AI widget: `frontend/app/aipanel/aipanel.tsx`
- Claude Code widget: `frontend/app/view/claudecode/claudecode-view.tsx`
- Widget system: `pkg/wconfig/defaultconfig/widgets.json`
- Tool types: Claude CLI documentation

---

## Next Steps

1. Review spec with team
2. Create implementation tasks in GitHub
3. Start Phase 1 development
4. User testing with early builds
5. Iterate based on feedback
