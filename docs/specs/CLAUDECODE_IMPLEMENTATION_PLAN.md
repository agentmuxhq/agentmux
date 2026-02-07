# Claude Code Pane Implementation Plan

**Status:** Ready for Development
**Approach:** A - Stream JSON Parsing
**Author:** AgentA
**Date:** 2026-02-07
**Based on:** CLAUDE_CODE_PANE_SPEC.md by agent5

---

## Overview

Implement a native Claude Code pane in WaveMux using `--output-format stream-json` to parse Claude Code's structured output and render it in a **terminal-native UI** - not a chat app. The pane should feel like a terminal with enhanced rendering: monospace font, dark background, sequential full-width content blocks, but with the benefit of proper markdown rendering, collapsible tool sections, and inline diffs.

---

## Phase 1: Skeleton Setup (Day 1)

### Goal
Create the basic pane infrastructure and verify it appears in WaveMux.

### Files to Create
```
frontend/app/view/claudecode/
├── claudecode.tsx          # ViewModel + View component
├── claudecode.scss         # Styles
├── claudecode-types.ts     # TypeScript interfaces
├── claudecode-parser.ts    # Stream parser (empty shell)
└── claudecode-message.tsx  # Message components (empty shell)
```

### Files to Modify
1. **frontend/app/block/block.tsx**
   - Import `ClaudeCodeViewModel`
   - Register: `BlockRegistry.set("claudecode", ClaudeCodeViewModel)`

2. **frontend/app/block/blockutil.tsx**
   - Add icon mapping: `if (view == "claudecode") return "message-bot"`
   - Add name mapping: `if (view == "claudecode") return "Claude Code"`

3. **frontend/types/custom.d.ts** (if needed)
   - Add `"claudecode"` to `ViewType` union

### Implementation Steps

#### Step 1.1: Create Basic Types
```typescript
// claudecode-types.ts
export interface ClaudeCodeViewModel {
    viewType: string;
    // ... basic structure
}

export interface ClaudeCodeMessage {
    role: "user" | "assistant" | "tool";
    content: string;
    timestamp: number;
}
```

#### Step 1.2: Create Minimal ViewModel
```typescript
// claudecode.tsx
import { ViewModel } from "@/app/view/view";
import { atom } from "jotai";

class ClaudeCodeViewModel implements ViewModel {
    viewType = "claudecode";
    viewIcon = atom("message-bot");
    viewName = atom("Claude Code");

    constructor() {
        // Empty for now
    }
}

const ClaudeCodeView = () => {
    return (
        <div className="claudecode-view">
            <h1>Claude Code Pane - Coming Soon</h1>
        </div>
    );
};

export { ClaudeCodeViewModel, ClaudeCodeView };
```

#### Step 1.3: Register in BlockRegistry
```typescript
// block.tsx
import { ClaudeCodeViewModel } from "@/app/view/claudecode/claudecode";

// In BlockRegistry initialization
BlockRegistry.set("claudecode", ClaudeCodeViewModel);
```

#### Step 1.4: Add to Launcher (Optional)
Check `frontend/app/view/vdom/vdom.tsx` or launcher widget to add "Claude Code" as a new pane type option.

### Validation
- [ ] Run `task dev`
- [ ] Pane appears in launcher or can be created via command
- [ ] Empty placeholder renders without errors
- [ ] DevTools console shows no errors

---

## Phase 2: Hidden Terminal Integration (Day 2-3)

### Goal
Spawn a hidden terminal running `claude --output-format stream-json` and verify process lifecycle.

### Key Challenges
1. Managing sub-block lifecycle
2. Capturing raw terminal output stream
3. Handling stdin/stdout communication

### Implementation Steps

#### Step 2.1: Create Sub-Block for Terminal

**Option A: Frontend-Driven (Simpler)**
```typescript
// In ClaudeCodeViewModel constructor
async initTerminal() {
    const termBlockId = await RpcApi.createSubBlock(this.blockId, {
        view: "term",
        controller: "cmd",
        "cmd": "claude",
        "cmd:args": ["--output-format", "stream-json"]
    });

    this.termBlockId = termBlockId;
    this.termNodeModel = // ... get node model for sub-block
}
```

**Option B: Backend-Driven (More Robust)**
Modify Go backend to automatically create the sub-block when a `claudecode` block is created.

```go
// pkg/waveobj/waveobj.go or relevant controller
func CreateClaudeCodeBlock(ctx context.Context, blockId string) error {
    // Create main block
    mainBlock := &Block{
        OID: blockId,
        Meta: map[string]any{
            "view": "claudecode",
        },
    }

    // Create sub-block for terminal
    subBlockId := uuid.New().String()
    termBlock := &Block{
        OID: subBlockId,
        ParentOID: blockId,
        Meta: map[string]any{
            "view": "term",
            "controller": "cmd",
            "cmd": "claude",
            "cmd:args": []string{"--output-format", "stream-json"},
        },
    }

    // Link them
    // ...
}
```

**Recommendation:** Start with Option A (frontend-driven) for faster iteration. Move to Option B if lifecycle issues arise.

#### Step 2.2: Access Terminal Output Stream

Study existing `TermWrap` implementation in `frontend/app/view/term/termwrap.ts`:

```typescript
// In ClaudeCodeViewModel
private termWrap: TermWrap;

async initTerminal() {
    // ... create sub-block ...

    // Create TermWrap for the hidden terminal
    this.termWrap = new TermWrap(
        this.termBlockId,
        this.termNodeModel,
        // ... options
    );

    // Subscribe to output stream
    const fileSubject = this.termWrap.getFileSubject(); // RxJS Observable
    fileSubject.subscribe((data: Uint8Array) => {
        const text = new TextDecoder().decode(data);
        this.onTerminalData(text);
    });
}

private onTerminalData(data: string) {
    console.log("Raw terminal output:", data);
    // Will wire to parser in Phase 3
}
```

**Key Question:** Does `TermWrap` expose the raw output stream directly? Need to investigate:
- Check `frontend/app/view/term/termwrap.ts`
- Look for file subject or terminal model APIs
- May need to tap into xterm.js `onData` event

#### Step 2.3: Implement Input Sending

```typescript
// In ClaudeCodeViewModel
async sendMessage(text: string) {
    await RpcApi.ControllerInputCommand(
        this.termBlockId,
        { text: text + "\n" }
    );
}
```

#### Step 2.4: Implement Interrupt/Reset

```typescript
async interrupt() {
    await RpcApi.ControllerInputCommand(
        this.termBlockId,
        { text: "\x03" }  // Ctrl+C
    );
}

async reset() {
    await this.sendMessage("/clear");
    // Or restart the terminal process
}
```

### Validation
- [ ] Hidden terminal spawns successfully
- [ ] `claude --output-format stream-json` process starts
- [ ] Console logs show raw JSON output when messages are sent
- [ ] Input from UI reaches the Claude process
- [ ] Interrupt (Ctrl+C) works
- [ ] Terminal can be toggled visible for debugging

---

## Phase 3: Stream Parser (Day 4-5)

### Goal
Parse NDJSON output from Claude Code and build a structured message list.

### Claude Code Stream Format

Based on testing, the stream-json format emits:

```json
{"type":"system","message":"Claude Code v1.2.3"}
{"type":"user_message","content":[{"type":"text","text":"Fix the bug"}]}
{"type":"assistant_message","content":[{"type":"text","text":"I'll look at it"}]}
{"type":"tool_use","id":"toolu_123","name":"Read","input":{"file_path":"app.ts"}}
{"type":"tool_result","tool_use_id":"toolu_123","content":"...file contents..."}
{"type":"result","usage":{"input_tokens":123,"output_tokens":456},"cost":0.0042}
```

**TODO:** Verify exact format by running `claude --output-format stream-json` locally and capturing output.

### Implementation Steps

#### Step 3.1: Define Event Types

```typescript
// claudecode-types.ts
export type ClaudeCodeEvent =
    | SystemEvent
    | UserMessageEvent
    | AssistantMessageEvent
    | ToolUseEvent
    | ToolResultEvent
    | ResultEvent
    | ErrorEvent;

export interface SystemEvent {
    type: "system";
    message: string;
}

export interface UserMessageEvent {
    type: "user_message";
    content: ContentBlock[];
}

export interface AssistantMessageEvent {
    type: "assistant_message";
    content: ContentBlock[];
    streaming?: boolean;  // If partial
}

export interface ToolUseEvent {
    type: "tool_use";
    id: string;
    name: string;
    input: Record<string, any>;
}

export interface ToolResultEvent {
    type: "tool_result";
    tool_use_id: string;
    content: string;
    is_error?: boolean;
}

export interface ResultEvent {
    type: "result";
    usage: {
        input_tokens: number;
        output_tokens: number;
    };
    cost: number;
}

export interface ErrorEvent {
    type: "error";
    message: string;
}

export type ContentBlock =
    | { type: "text"; text: string }
    | { type: "tool_use"; id: string; name: string; input: any };
```

#### Step 3.2: Implement NDJSON Parser

```typescript
// claudecode-parser.ts
export class ClaudeCodeStreamParser {
    private buffer: string = "";
    private onEventCallback: (event: ClaudeCodeEvent) => void;

    constructor(onEvent: (event: ClaudeCodeEvent) => void) {
        this.onEventCallback = onEvent;
    }

    /**
     * Feed raw terminal output into the parser.
     * Handles incomplete lines, non-JSON text, and streaming.
     */
    feedData(data: string): void {
        this.buffer += data;

        // Split by newlines
        const lines = this.buffer.split("\n");

        // Keep last incomplete line in buffer
        this.buffer = lines.pop() || "";

        for (const line of lines) {
            if (line.trim() === "") continue;

            try {
                const event = JSON.parse(line) as ClaudeCodeEvent;
                this.onEventCallback(event);
            } catch (err) {
                // Non-JSON output (startup banner, errors, etc.)
                console.warn("Failed to parse JSON line:", line, err);

                // Optionally emit as system message
                this.onEventCallback({
                    type: "system",
                    message: line
                });
            }
        }
    }

    reset(): void {
        this.buffer = "";
    }
}
```

#### Step 3.3: Wire Parser to ViewModel

```typescript
// claudecode.tsx
class ClaudeCodeViewModel implements ViewModel {
    private parser: ClaudeCodeStreamParser;
    messages = atom<ClaudeCodeMessage[]>([]);
    sessionMeta = atom<SessionMeta>({ model: "sonnet", tokens: 0, cost: 0 });

    constructor() {
        this.parser = new ClaudeCodeStreamParser(
            this.handleEvent.bind(this)
        );

        // ... init terminal ...
    }

    private onTerminalData(data: string) {
        this.parser.feedData(data);
    }

    private handleEvent(event: ClaudeCodeEvent) {
        console.log("Parsed event:", event);

        switch (event.type) {
            case "user_message":
                this.addMessage({
                    role: "user",
                    content: event.content,
                    timestamp: Date.now()
                });
                break;

            case "assistant_message":
                this.addMessage({
                    role: "assistant",
                    content: event.content,
                    timestamp: Date.now()
                });
                break;

            case "tool_use":
                this.addToolUse(event);
                break;

            case "tool_result":
                this.addToolResult(event);
                break;

            case "result":
                this.updateSessionMeta(event);
                break;

            case "system":
                // Log or display in UI
                console.log("System:", event.message);
                break;
        }
    }

    private addMessage(msg: ClaudeCodeMessage) {
        const msgs = [...this.messages.init, msg];
        this.messages.init = msgs;
    }

    private addToolUse(event: ToolUseEvent) {
        // Add as a separate message or append to last assistant message
        this.addMessage({
            role: "tool",
            toolName: event.name,
            toolId: event.id,
            toolInput: event.input,
            content: [],
            timestamp: Date.now(),
            isCollapsed: true
        });
    }

    private addToolResult(event: ToolResultEvent) {
        // Find matching tool_use and attach result
        const msgs = this.messages.init;
        const toolMsg = msgs.find(m => m.toolId === event.tool_use_id);
        if (toolMsg) {
            toolMsg.toolResult = event.content;
            toolMsg.toolError = event.is_error;
            this.messages.init = [...msgs]; // Trigger update
        }
    }

    private updateSessionMeta(event: ResultEvent) {
        this.sessionMeta.init = {
            model: this.sessionMeta.init.model,
            tokens: event.usage.input_tokens + event.usage.output_tokens,
            cost: event.cost
        };
    }
}
```

### Validation
- [ ] Events are parsed correctly from raw JSON
- [ ] Messages appear in `messages` atom
- [ ] Tool use events are captured and linked to results
- [ ] Session metadata (tokens, cost) updates
- [ ] Console logs show structured events
- [ ] Malformed JSON doesn't crash the parser

---

## Phase 4: Terminal-Native UI Rendering (Day 6-8)

### Goal
Build a UI that **looks and feels like a terminal**, not a chat app. No bubbles, no avatars, no SMS-style left/right alignment. Think of it as a **scrolling terminal log with enhanced rendering**.

### Design Philosophy

The UI should feel like you're reading a terminal session, but with superpowers:

```
┌─ Claude Code ──────────── opus-4 │ 12.3k tokens │ $0.42 ─┐
│                                                            │
│  ❯ Fix the auth bug in login.ts                            │  ← User prompt (looks like shell input)
│                                                            │
│  I'll look at the login file to find the issue.            │  ← Assistant text (full-width, no bubble)
│                                                            │
│  ▸ Read login.ts                                           │  ← Collapsed tool block (one-liner)
│  ▸ Read auth-utils.ts                                      │
│                                                            │
│  Found the issue on line 42. The token validation          │  ← More assistant text
│  skips the expiry check when `remember_me` is set.         │
│                                                            │
│  ▾ Edit login.ts                                           │  ← Expanded tool block
│  │  @@ -40,6 +40,8 @@                                     │
│  │  - if (token.valid) {                                   │  ← Inline diff (red/green)
│  │  + if (token.valid && !token.expired) {                 │
│  │    ...                                                  │
│                                                            │
│  The fix ensures expired tokens are rejected even          │
│  when remember_me is enabled.                              │
│                                                            │
│  ─────────────────────────────────────────────────         │  ← Turn separator
│                                                            │
│  ❯ Now add a test for that case                            │  ← Next user prompt
│                                                            │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━ ● streaming...                  │  ← Streaming indicator (not dots)
│                                                            │
│  ❯ _                                                       │  ← Input line (cursor blinks)
└────────────────────────────────────────────────────────────┘
```

### Key UI Principles

1. **Full-width blocks** - Everything flows top to bottom, full width. No alignment tricks.
2. **Monospace font** - Same font as the terminal. This IS a terminal experience.
3. **User prompts look like shell input** - Prefixed with `❯` (or configurable prompt char), slightly highlighted background.
4. **Assistant text is plain** - Just rendered markdown with syntax highlighting. No avatar, no bubble, no container. It's like reading terminal output.
5. **Tool blocks are one-line summaries** - `▸ Read login.ts` collapsed, `▾ Read login.ts` expanded. Click to toggle. Looks like a tree node, not a card.
6. **Turn separators** - A thin horizontal rule between conversation turns (user prompt + full assistant response).
7. **Streaming indicator** - A pulsing cursor or subtle bar, not bouncing dots.
8. **Color scheme** - Terminal colors. Green for user prompts, default for assistant text, dim for tool summaries, red/green for diffs.
9. **Input line** - At the bottom, looks like a terminal prompt line. `❯ ` prefix with blinking cursor.

### Component Structure (Revised)

```
ClaudeCodeView (main container - terminal-styled)
├── ClaudeCodeLog (scrollable log area)
│   ├── ConversationTurn
│   │   ├── UserPrompt         (❯ prefixed, highlighted)
│   │   ├── AssistantBlock     (full-width rendered markdown)
│   │   │   ├── TextSegment    (plain markdown text)
│   │   │   └── ToolBlock      (collapsible one-liner)
│   │   │       └── ToolDetail (diff, file content, command output)
│   │   └── TurnSeparator      (thin horizontal rule)
│   └── StreamingCursor        (pulsing indicator)
├── InputLine (terminal prompt at bottom)
└── StatusBar (footer: controls + session info)
```

### Implementation Steps

#### Step 4.1: Conversation Log Container

```typescript
// claudecode.tsx
const ClaudeCodeLog = ({ turns }: { turns: ConversationTurn[] }) => {
    const scrollRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        scrollRef.current?.scrollTo({
            top: scrollRef.current.scrollHeight,
            behavior: "smooth"
        });
    }, [turns]);

    return (
        <div className="cc-log" ref={scrollRef}>
            {turns.map((turn, i) => (
                <ConversationTurnView key={i} turn={turn} />
            ))}
        </div>
    );
};
```

#### Step 4.2: Conversation Turn (User Prompt + Assistant Response)

```typescript
// claudecode-message.tsx
const ConversationTurnView = ({ turn }: { turn: ConversationTurn }) => {
    return (
        <div className="cc-turn">
            {/* User prompt - looks like shell input */}
            <div className="cc-prompt">
                <span className="cc-prompt-char">❯</span>
                <span className="cc-prompt-text">{turn.userInput}</span>
            </div>

            {/* Assistant response - sequential blocks */}
            {turn.blocks.map((block, i) => {
                switch (block.type) {
                    case "text":
                        return (
                            <div key={i} className="cc-text">
                                <Markdown text={block.text} />
                            </div>
                        );
                    case "tool":
                        return <ToolBlock key={i} tool={block} />;
                    default:
                        return null;
                }
            })}

            {/* Turn separator */}
            <div className="cc-turn-separator" />
        </div>
    );
};
```

#### Step 4.3: Tool Block (Collapsible One-Liner)

```typescript
// claudecode-message.tsx
const ToolBlock = ({ tool }: { tool: ToolBlockData }) => {
    const [expanded, setExpanded] = useState(false);

    // One-line summary: "▸ Read  src/login.ts"
    const summary = getToolOneLiner(tool.name, tool.input);

    return (
        <div className={clsx("cc-tool", { expanded })} onClick={() => setExpanded(!expanded)}>
            <div className="cc-tool-line">
                <span className="cc-tool-chevron">{expanded ? "▾" : "▸"}</span>
                <span className="cc-tool-name">{tool.name}</span>
                <span className="cc-tool-summary">{summary}</span>
                {tool.isError && <span className="cc-tool-error">✗</span>}
                {!tool.isError && tool.result && <span className="cc-tool-ok">✓</span>}
            </div>

            {expanded && (
                <div className="cc-tool-detail">
                    {tool.name === "Edit" && tool.result ? (
                        <DiffBlock input={tool.input} result={tool.result} />
                    ) : tool.name === "Bash" ? (
                        <BashOutputBlock input={tool.input} result={tool.result} />
                    ) : (
                        <pre className="cc-tool-output">
                            {tool.result || JSON.stringify(tool.input, null, 2)}
                        </pre>
                    )}
                </div>
            )}
        </div>
    );
};

function getToolOneLiner(name: string, input: any): string {
    switch (name) {
        case "Read":   return input.file_path;
        case "Write":  return input.file_path;
        case "Edit":   return input.file_path;
        case "Bash":   return input.command?.length > 60
                              ? input.command.substring(0, 60) + "…"
                              : input.command;
        case "Glob":   return input.pattern;
        case "Grep":   return `/${input.pattern}/ ${input.path || ""}`;
        case "Task":   return input.description || "";
        default:       return "";
    }
}
```

#### Step 4.4: Diff Rendering for Edit Tool

```typescript
// claudecode-message.tsx
const DiffBlock = ({ input, result }: { input: any; result: string }) => {
    // Render as terminal-style diff (red/green lines)
    return (
        <pre className="cc-diff">
            <div className="cc-diff-header">{input.file_path}</div>
            {result.split("\n").map((line, i) => {
                const cls = line.startsWith("+") ? "cc-diff-add"
                          : line.startsWith("-") ? "cc-diff-del"
                          : line.startsWith("@") ? "cc-diff-hunk"
                          : "cc-diff-ctx";
                return <div key={i} className={cls}>{line}</div>;
            })}
        </pre>
    );
};
```

#### Step 4.5: Bash Output Block

```typescript
const BashOutputBlock = ({ input, result }: { input: any; result: string }) => {
    return (
        <div className="cc-bash">
            <div className="cc-bash-cmd">
                <span className="cc-bash-dollar">$</span> {input.command}
            </div>
            {result && (
                <pre className="cc-bash-output">{result}</pre>
            )}
        </div>
    );
};
```

#### Step 4.6: Streaming Indicator

```typescript
// Terminal-style: pulsing block cursor or thin progress bar
const StreamingCursor = () => {
    return (
        <div className="cc-streaming">
            <span className="cc-streaming-cursor">█</span>
        </div>
    );
};
```

### Validation
- [ ] UI feels like a terminal session, not a chat app
- [ ] User prompts display with `❯` prefix
- [ ] Assistant text renders as full-width markdown
- [ ] Tool blocks show as one-line summaries
- [ ] Tool blocks expand/collapse with `▸`/`▾` chevrons
- [ ] Diffs render with red/green terminal colors
- [ ] Bash commands show with `$` prefix and output below
- [ ] Streaming uses block cursor, not bouncing dots
- [ ] Monospace font throughout
- [ ] Turn separators between conversation turns
- [ ] Auto-scroll to bottom on new content

---

## Phase 5: Input Line (Day 9)

### Goal
Build a terminal-style input line at the bottom that looks like a shell prompt.

### Implementation Steps

#### Step 5.1: Input Component (Terminal Prompt Style)

```typescript
// claudecode-input.tsx
// Looks like a terminal prompt: ❯ type here_

const ClaudeCodeInput = ({ model }: { model: ClaudeCodeViewModel }) => {
    const [inputText, setInputText] = useState("");
    const [isStreaming] = useAtom(model.isStreaming);
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const handleSend = useCallback(() => {
        if (!inputText.trim() || isStreaming) return;
        model.sendMessage(inputText);
        setInputText("");
    }, [inputText, isStreaming, model]);

    const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
        if (e.key === "Escape") {
            model.interrupt();
        }
    }, [handleSend, model]);

    // Auto-expand textarea
    useEffect(() => {
        const el = textareaRef.current;
        if (el) {
            el.style.height = "auto";
            el.style.height = el.scrollHeight + "px";
        }
    }, [inputText]);

    return (
        <div className="cc-input">
            <span className="cc-input-prompt">❯</span>
            <textarea
                ref={textareaRef}
                className="cc-input-textarea"
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={isStreaming ? "" : ""}
                disabled={isStreaming}
                rows={1}
            />
        </div>
    );
};
```

No send button. Enter to send (like a terminal). Shift+Enter for newline. The `❯` prompt char matches the user prompts in the log above.

### Validation
- [ ] Input line renders at bottom with `❯` prefix
- [ ] Looks like a terminal prompt, not a chat input
- [ ] Enter sends, Shift+Enter for newline
- [ ] Escape interrupts current generation
- [ ] Auto-expands for multiline input
- [ ] Input disabled during streaming (cursor stays visible but dimmed)
- [ ] No send button (terminal doesn't have send buttons)

---

## Phase 6: Header & Footer (Day 10)

### Goal
Add metadata to the header and control buttons to the footer.

### Implementation Steps

#### Step 6.1: Header Metadata

```typescript
// In ClaudeCodeViewModel
viewText = atom<HeaderElem[]>((get) => {
    const meta = get(this.sessionMeta);
    return [
        {
            elemtype: "text",
            text: meta.model,
        },
        {
            elemtype: "text",
            text: `${(meta.tokens / 1000).toFixed(1)}k tokens`,
        },
        {
            elemtype: "text",
            text: `$${meta.cost.toFixed(3)}`,
        },
    ];
});

endIconButtons = atom<IconButtonDecl[]>([
    {
        elemtype: "iconbutton",
        icon: "terminal",
        title: "Toggle Terminal",
        click: () => this.toggleTerminal(),
    },
    {
        elemtype: "iconbutton",
        icon: "rotate-right",
        title: "Reset Session",
        click: () => this.reset(),
    },
]);
```

#### Step 6.2: Status Bar (Terminal-Style Footer)

```typescript
// claudecode.tsx
// Like a vim/tmux status bar, not a button bar

const ClaudeCodeStatusBar = ({ model }: { model: ClaudeCodeViewModel }) => {
    const [showTerminal] = useAtom(model.showTerminal);
    const [isStreaming] = useAtom(model.isStreaming);
    const [meta] = useAtom(model.sessionMeta);

    return (
        <div className="cc-statusbar">
            <span className="cc-status-item">
                {isStreaming ? "● streaming" : "○ idle"}
            </span>
            <span className="cc-status-item">{meta.model}</span>
            <span className="cc-status-item">{(meta.tokens / 1000).toFixed(1)}k</span>
            <span className="cc-status-item">${meta.cost.toFixed(3)}</span>

            <span className="cc-status-spacer" />

            <button className="cc-status-btn" onClick={() => model.toggleTerminal()}>
                {showTerminal ? "[chat]" : "[term]"}
            </button>
            <button className="cc-status-btn" onClick={() => model.interrupt()} disabled={!isStreaming}>
                [^C]
            </button>
            <button className="cc-status-btn" onClick={() => model.reset()}>
                [reset]
            </button>
        </div>
    );
};
```

Status bar items use square brackets like terminal shortcuts. `[^C]` for interrupt, `[term]` to toggle terminal view.

### Validation
- [ ] Status bar shows at bottom like tmux/vim status line
- [ ] Model name, token count, cost visible
- [ ] Values update after each response
- [ ] `[term]` toggle shows raw terminal
- [ ] `[^C]` interrupts current generation
- [ ] `[reset]` clears the session
- [ ] Streaming state indicator works (● / ○)

---

## Phase 7: Terminal Toggle (Day 11)

### Goal
Allow switching between chat UI and raw terminal for debugging.

### Implementation Steps

#### Step 7.1: CSS Layout

```scss
// claudecode.scss
.claudecode-view {
    display: flex;
    flex-direction: column;
    height: 100%;

    .claudecode-chat {
        display: flex;
        flex-direction: column;
        flex: 1;
        overflow: hidden;

        &.hidden {
            display: none;
        }
    }

    .claudecode-terminal {
        flex: 1;
        overflow: hidden;

        &.hidden {
            display: none;
        }
    }
}
```

#### Step 7.2: Terminal Visibility Toggle

```typescript
// In ClaudeCodeViewModel
showTerminal = atom<boolean>(false);

toggleTerminal() {
    this.showTerminal.init = !this.showTerminal.init;
}
```

#### Step 7.3: Render Main View with Toggle

```typescript
// claudecode.tsx
const ClaudeCodeView = ({ model }: { model: ClaudeCodeViewModel }) => {
    const [showTerminal] = useAtom(model.showTerminal);
    const turns = useAtomValue(model.turns);
    const isStreaming = useAtomValue(model.isStreaming);

    return (
        <div className="claudecode-view">
            {/* Enhanced terminal log (default) */}
            <div className={clsx("cc-log-container", { hidden: showTerminal })}>
                <ClaudeCodeLog turns={turns} />
                {isStreaming && <StreamingCursor />}
                <ClaudeCodeInput model={model} />
            </div>

            {/* Raw terminal (toggle) */}
            <div className={clsx("cc-raw-terminal", { hidden: !showTerminal })}>
                <SubBlock nodeModel={model.termNodeModel} />
            </div>

            <ClaudeCodeStatusBar model={model} />
        </div>
    );
};
```

The default view is the enhanced terminal log. Toggle `[term]` in the status bar to see the raw JSON terminal output underneath (useful for debugging).

### Validation
- [ ] Toggle button switches between chat and terminal
- [ ] Terminal output is visible when toggled
- [ ] Chat UI hidden when terminal shown (and vice versa)
- [ ] Terminal is interactive (can type directly)
- [ ] Switching doesn't break state

---

## Phase 8: Styling & Polish (Day 12-14)

### Goal
Make it look professional and match WaveMux's theme.

### Implementation Steps

#### Step 8.1: SCSS Structure (Terminal-Native)

```scss
// claudecode.scss
// Design: Terminal-native. Monospace everywhere. No bubbles. No avatars.
// Think: enhanced terminal output, not a chat app.

.claudecode-view {
    background: var(--term-bg-color, #1a1b26);
    color: var(--term-fg-color, #c0caf5);
    font-family: var(--termfontfamily, "JetBrains Mono", "Fira Code", monospace);
    font-size: var(--termfontsize, 13px);
    line-height: 1.6;
    display: flex;
    flex-direction: column;
    height: 100%;

    // === Scrollable Log ===
    .cc-log {
        flex: 1;
        overflow-y: auto;
        padding: 12px 16px;
    }

    // === Conversation Turn (prompt + response) ===
    .cc-turn {
        margin-bottom: 4px;
    }

    .cc-turn-separator {
        border: none;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
        margin: 16px 0;
    }

    // === User Prompt ===
    .cc-prompt {
        display: flex;
        gap: 8px;
        padding: 6px 8px;
        margin-bottom: 8px;
        background: rgba(255, 255, 255, 0.03);
        border-left: 2px solid #7aa2f7;

        .cc-prompt-char {
            color: #7aa2f7;       // Blue prompt character
            font-weight: bold;
            user-select: none;
        }

        .cc-prompt-text {
            color: #c0caf5;
            white-space: pre-wrap;
        }
    }

    // === Assistant Text ===
    .cc-text {
        padding: 2px 8px 8px 8px;
        color: #a9b1d6;

        // Markdown overrides for terminal feel
        p { margin: 4px 0; }

        code {
            background: rgba(255, 255, 255, 0.06);
            padding: 1px 4px;
            border-radius: 2px;
            color: #bb9af7;
        }

        pre {
            background: rgba(0, 0, 0, 0.3);
            padding: 8px 12px;
            border-radius: 3px;
            border-left: 2px solid rgba(255, 255, 255, 0.08);
            overflow-x: auto;
            margin: 6px 0;

            code {
                background: none;
                padding: 0;
                color: inherit;
            }
        }

        strong { color: #c0caf5; }
        em { color: #9ece6a; }
        a { color: #7aa2f7; text-decoration: underline; }

        ul, ol {
            margin: 4px 0;
            padding-left: 20px;
        }
    }

    // === Tool Blocks (collapsible one-liners) ===
    .cc-tool {
        margin: 2px 0;
        cursor: pointer;
        user-select: none;

        &:hover .cc-tool-line {
            background: rgba(255, 255, 255, 0.04);
        }

        .cc-tool-line {
            display: flex;
            align-items: center;
            gap: 6px;
            padding: 3px 8px;
            border-radius: 2px;
            transition: background 0.1s;
        }

        .cc-tool-chevron {
            color: #565f89;
            width: 12px;
            font-size: 11px;
        }

        .cc-tool-name {
            color: #9ece6a;       // Green for tool names
            font-weight: 600;
        }

        .cc-tool-summary {
            color: #565f89;       // Dim for paths/commands
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }

        .cc-tool-ok {
            color: #9ece6a;
            margin-left: auto;
        }

        .cc-tool-error {
            color: #f7768e;
            margin-left: auto;
        }

        // Expanded detail area
        .cc-tool-detail {
            margin: 4px 0 4px 20px;
            padding: 6px 10px;
            background: rgba(0, 0, 0, 0.2);
            border-left: 1px solid rgba(255, 255, 255, 0.06);
            border-radius: 2px;
            font-size: 12px;

            pre {
                margin: 0;
                white-space: pre-wrap;
                word-break: break-all;
            }
        }
    }

    // === Diff Rendering ===
    .cc-diff {
        margin: 0;
        font-size: 12px;

        .cc-diff-header {
            color: #565f89;
            padding-bottom: 4px;
            border-bottom: 1px solid rgba(255, 255, 255, 0.06);
            margin-bottom: 4px;
        }

        .cc-diff-add {
            color: #9ece6a;
            background: rgba(158, 206, 106, 0.08);
        }

        .cc-diff-del {
            color: #f7768e;
            background: rgba(247, 118, 142, 0.08);
        }

        .cc-diff-hunk {
            color: #7aa2f7;
        }

        .cc-diff-ctx {
            color: #565f89;
        }
    }

    // === Bash Command Output ===
    .cc-bash {
        .cc-bash-cmd {
            display: flex;
            gap: 6px;
            color: #c0caf5;

            .cc-bash-dollar {
                color: #9ece6a;
                user-select: none;
            }
        }

        .cc-bash-output {
            color: #565f89;
            margin: 2px 0 0 0;
            font-size: 12px;
            max-height: 300px;
            overflow-y: auto;
        }
    }

    // === Input Line ===
    .cc-input {
        display: flex;
        align-items: flex-end;
        gap: 0;
        padding: 8px 16px 12px;
        border-top: 1px solid rgba(255, 255, 255, 0.06);
        background: rgba(0, 0, 0, 0.15);

        .cc-input-prompt {
            color: #7aa2f7;
            font-weight: bold;
            padding: 6px 8px 6px 0;
            user-select: none;
        }

        .cc-input-textarea {
            flex: 1;
            background: transparent;
            border: none;
            outline: none;
            color: #c0caf5;
            font-family: inherit;
            font-size: inherit;
            line-height: inherit;
            resize: none;
            min-height: 22px;
            max-height: 200px;
            caret-color: #7aa2f7;

            &::placeholder {
                color: #3b4261;
            }
        }
    }

    // === Streaming Indicator ===
    .cc-streaming {
        padding: 4px 8px;

        .cc-streaming-cursor {
            color: #7aa2f7;
            animation: blink 1s step-end infinite;
        }
    }

    // === Status Bar (Footer) ===
    .cc-statusbar {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 4px 16px;
        background: rgba(0, 0, 0, 0.2);
        border-top: 1px solid rgba(255, 255, 255, 0.06);
        font-size: 11px;
        color: #565f89;

        .cc-status-item {
            display: flex;
            align-items: center;
            gap: 4px;
        }

        .cc-status-btn {
            background: none;
            border: none;
            color: #7aa2f7;
            cursor: pointer;
            font-family: inherit;
            font-size: inherit;
            padding: 2px 6px;
            border-radius: 2px;

            &:hover {
                background: rgba(255, 255, 255, 0.06);
            }

            &:disabled {
                color: #3b4261;
                cursor: default;
            }
        }

        .cc-status-spacer {
            flex: 1;
        }
    }
}

@keyframes blink {
    50% { opacity: 0; }
}
```

#### Step 8.2: Dark/Light Theme Support

Ensure all custom CSS variables are defined in both themes:
- Check `frontend/app/app.scss` for theme definitions
- Test in both light and dark modes

#### Step 8.3: Animations

Add smooth transitions:
- Message fade-in
- Tool block expand/collapse
- Streaming dots animation

#### Step 8.4: Responsive Design

Ensure the pane works at different sizes (small, medium, large).

### Validation
- [ ] Matches WaveMux visual style
- [ ] Works in light and dark themes
- [ ] Animations are smooth
- [ ] No visual glitches or layout breaks
- [ ] Looks good at all pane sizes

---

## Phase 9: Error Handling (Day 15)

### Goal
Handle edge cases and errors gracefully.

### Scenarios to Handle

1. **Claude Code not installed**
   - Show error message: "Claude Code CLI not found. Install from claude.ai"
   - Offer fallback to WaveAI pane

2. **Process crashes**
   - Detect when `claude` process exits unexpectedly
   - Show error state with restart button

3. **Parsing errors**
   - Non-JSON output mixed in stream
   - Malformed JSON events
   - Fallback: Display raw text as system message

4. **Interactive prompts** (Y/n confirmations)
   - Detect prompt patterns in output
   - Show inline button UI or fall through to terminal

5. **Large outputs**
   - Tool results > 10KB
   - Truncate with "Show more" button

6. **Network issues** (if remote connection)
   - Handle slow/disconnected remote connections

### Implementation

```typescript
// claudecode.tsx
const ClaudeCodeErrorState = ({ error, onRetry }: { error: string; onRetry: () => void }) => {
    return (
        <div className="claudecode-error">
            <i className="fa-solid fa-triangle-exclamation" />
            <h3>Error</h3>
            <p>{error}</p>
            <button onClick={onRetry}>
                <i className="fa-solid fa-rotate-right" />
                Retry
            </button>
        </div>
    );
};
```

### Validation
- [ ] Error states render properly
- [ ] Retry works for recoverable errors
- [ ] Parser handles malformed input
- [ ] Large outputs are truncated
- [ ] Process crashes are detected

---

## Phase 10: Testing & Refinement (Day 16-18)

### Manual Testing Checklist

- [ ] Create new claudecode pane
- [ ] Send simple message, verify response
- [ ] Test tool use (Read, Edit, Bash)
- [ ] Verify tool blocks collapse/expand
- [ ] Test interrupt (Ctrl+C)
- [ ] Test reset (/clear)
- [ ] Toggle terminal view
- [ ] Check token/cost display
- [ ] Test long conversation (20+ messages)
- [ ] Test large file reads (>1000 lines)
- [ ] Test rapid input (multiple messages quickly)
- [ ] Test in light and dark themes
- [ ] Test at different pane sizes
- [ ] Test with remote connection (if applicable)

### Automated Testing

```typescript
// __tests__/claudecode.test.ts
describe("ClaudeCodeViewModel", () => {
    test("parses user message event", () => {
        const parser = new ClaudeCodeStreamParser(/* ... */);
        const event = '{"type":"user_message","content":[{"type":"text","text":"Hello"}]}';
        // Assert message added
    });

    test("handles tool use event", () => {
        // Assert tool block created
    });

    test("handles malformed JSON", () => {
        // Assert no crash
    });
});
```

### Performance Testing

- [ ] Measure render time for 100 messages
- [ ] Check memory usage over long session
- [ ] Profile parser performance with large JSON events

---

## Known Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Stream-json format changes in future Claude Code versions | Medium | High | Version-pin format, add fallback to terminal view |
| Interactive prompts break chat UI | Medium | Medium | Detect patterns, show terminal for prompts |
| Large tool outputs freeze UI | Low | Medium | Virtualize message list, truncate outputs |
| Sub-block lifecycle bugs | Medium | High | Thorough testing, use existing SubBlock patterns |
| Parser overwhelmed by rapid output | Low | Low | Debounce parsing, batch updates |

---

## Future Enhancements (Post-MVP)

1. **File path click-through** - Click paths to open in preview/editor
2. **Inline diff viewer** - For Edit tool results
3. **Image rendering** - Display images in chat
4. **Session persistence** - Save/restore conversations
5. **Multi-tab** - Multiple Claude sessions in one pane
6. **MCP server status** - Show connected MCP tools
7. **Custom theme** - Claude-branded color scheme
8. **Export conversation** - Save as Markdown/HTML
9. **Search messages** - Find text in conversation history
10. **Voice input** (ambitious) - Speak to Claude

---

## Success Criteria

- [ ] Pane can be created and appears in launcher
- [ ] Claude Code process starts and runs in hidden terminal
- [ ] Messages are sent and responses received
- [ ] Tool use blocks render and are collapsible
- [ ] Header shows model, tokens, cost
- [ ] Input area works (Enter to send, multiline support)
- [ ] Interrupt and reset functions work
- [ ] Terminal toggle works for debugging
- [ ] Styling matches WaveMux theme
- [ ] No crashes or critical bugs
- [ ] Performance is acceptable (no lag)

---

## Estimated Timeline

| Phase | Days | Notes |
|-------|------|-------|
| 1. Skeleton | 1 | Quick setup |
| 2. Terminal | 2-3 | Trickiest part - sub-block lifecycle |
| 3. Parser | 2 | Well-defined problem |
| 4. Chat UI | 3 | Most code volume |
| 5. Input | 1 | Straightforward |
| 6. Header/Footer | 1 | Minor |
| 7. Terminal Toggle | 1 | Simple |
| 8. Styling | 3 | Time-consuming polish |
| 9. Error Handling | 1 | Edge cases |
| 10. Testing | 3 | Critical for quality |
| **Total** | **18 days** | ~3.5 weeks solo |

With parallel work or multiple agents: **~2 weeks**

---

## Next Steps

1. **Verify `claude --output-format stream-json`** - Run locally, capture exact format
2. **Study existing code:**
   - `frontend/app/view/term/termwrap.ts` - Terminal management
   - `frontend/app/view/waveai/waveai.tsx` - Chat UI patterns
   - `frontend/app/block/block.tsx` - ViewModel registration
3. **Set up dev environment:**
   - `task dev` running
   - Chrome DevTools open for debugging
4. **Create feature branch:** `agenta/claudecode-pane`
5. **Start with Phase 1**

---

## Questions to Resolve

- [ ] Exact format of `claude --output-format stream-json` output?
- [ ] Does TermWrap expose raw output stream or only xterm.js?
- [ ] How does SubBlock handle process lifecycle?
- [ ] Are there existing Markdown/diff components to reuse?
- [ ] Should we handle `/remember` and other slash commands?
- [ ] Should the pane auto-start Claude or wait for first message?
- [ ] What's the best way to handle file path clicks?

---

**Ready to implement!** Starting with Phase 1 to validate the basic structure.
