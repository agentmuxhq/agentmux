# POC: Agent Widget Interactive Initialization

**Date:** 2026-02-16
**Status:** Proof of Concept Spec
**Goal:** Relay Claude Code's initialization questions through agent pane UI and capture user responses

---

## Overview

Prove that the agent widget can:
1. Spawn a fresh Claude Code instance on "Connect" button click
2. Capture Claude's initialization prompts (light/dark mode, login, etc.)
3. Display prompts to user in agent pane UI
4. Accept user input through custom UI controls
5. Relay responses back to Claude Code's STDIN
6. Successfully complete initialization flow

**Why This Matters:**
- Validates bidirectional I/O relay works end-to-end
- Proves we can intercept and handle Claude's interactive prompts
- Foundation for permission approval UI, multi-agent messaging, and other interactive features

---

## Current State Analysis

### What Works ✅

```typescript
// Agent widget already has:
1. Process spawning via controller (cmd: "claude")
2. Output streaming via FileSubject (claude-code.jsonl)
3. Input relay via RpcApi.ControllerInputCommand()
4. Connection status UI (ConnectionStatus.tsx)
```

### What's Missing ❌

```typescript
// Need to add:
1. Interactive initialization UI (question prompts)
2. Detection of Claude's setup questions
3. Custom input controls for specific prompts
4. State machine for initialization flow
5. Fresh environment guarantee (--clear-chat or similar)
```

---

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  Agent Widget UI (React)                                   │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  ConnectionStatus                                     │ │
│  │  ┌─────────────────────────────────────────────────┐ │ │
│  │  │  [Connect] ← User clicks                        │ │ │
│  │  └─────────────────────────────────────────────────┘ │ │
│  └──────────────────────────────────────────────────────┘ │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  InitializationPrompt                                │ │
│  │  ┌─────────────────────────────────────────────────┐ │ │
│  │  │  Question: "Choose theme"                       │ │ │
│  │  │  [ Light ]  [ Dark ]  ← User selects           │ │ │
│  │  └─────────────────────────────────────────────────┘ │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  AgentViewModel (TypeScript)                               │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  handleConnect() {                                   │ │
│  │    1. startClaudeCodeProcess()                       │ │
│  │    2. monitorInitQuestions()                         │ │
│  │    3. setState("initializing")                       │ │
│  │  }                                                   │ │
│  │                                                      │ │
│  │  detectInitQuestion(output) {                        │ │
│  │    if (matches theme prompt) return "theme"          │ │
│  │    if (matches login prompt) return "login"          │ │
│  │  }                                                   │ │
│  │                                                      │ │
│  │  respondToPrompt(answer) {                           │ │
│  │    sendInput(answer + "\n")                          │ │
│  │  }                                                   │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  I/O Relay (FileSubject + RpcApi)                         │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Output: Subscribe to process output                 │ │
│  │  Input: Send to process STDIN                        │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  ShellController (Go Backend)                              │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  Start() {                                           │ │
│  │    cmd = "claude --output-format stream-json"        │ │
│  │    // Maybe add: --clear-chat                        │ │
│  │    spawnProcess(cmd)                                 │ │
│  │  }                                                   │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  Claude Code Process                                       │
│  ┌──────────────────────────────────────────────────────┐ │
│  │  1. Starts up                                        │ │
│  │  2. Asks: "Choose theme (light/dark):"              │ │
│  │  3. Waits for STDIN                                  │ │
│  │  4. Asks: "Login? (y/n):"                           │ │
│  │  5. Waits for STDIN                                  │ │
│  │  6. Ready to accept prompts                          │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

---

## Initialization Flow

### Phase 1: Spawn Fresh Instance

**User Action:**
```typescript
// User clicks "Connect" in ConnectionStatus component
<button onClick={handleConnect}>Connect</button>
```

**ViewModel Action:**
```typescript
async handleConnect() {
    // 1. Set state to "initializing"
    globalStore.set(this.atoms.initStateAtom, {
        phase: "spawning",
        message: "Starting Claude Code..."
    });

    // 2. Start controller (spawns process)
    await RpcApi.ControllerResyncCommand(TabRpcClient, {
        tabid: globalStore.get(atoms.staticTabId),
        blockid: this.blockId,
        forcerestart: true  // Ensure fresh instance
    });

    // 3. Begin monitoring for init questions
    this.initMonitor = new InitializationMonitor(this.atoms);
    this.initMonitor.start();
}
```

**Backend Action:**
```go
// shellcontroller.go - Already handles this
// Just ensure we're starting fresh (no cached session)

// FUTURE: May need to add --clear-chat flag if available
```

---

### Phase 2: Detect Questions

**Output Patterns to Detect:**

```typescript
interface InitQuestion {
    type: "theme" | "login" | "other";
    prompt: string;
    options?: string[];
    expectsInput: boolean;
}

// Pattern matching on raw output (before NDJSON parsing)
const INIT_PATTERNS = {
    theme: /Choose (your )?theme.*\(light\/dark\)/i,
    login: /Log in.*\?.*\(y\/n\)/i,
    generic: /\[.*\]\s*:/,  // Catch generic prompts
};

class InitializationMonitor {
    private buffer: string = "";

    handleRawOutput(chunk: string) {
        this.buffer += chunk;

        // Check for theme question
        if (INIT_PATTERNS.theme.test(this.buffer)) {
            this.emitQuestion({
                type: "theme",
                prompt: "Choose your theme",
                options: ["light", "dark"],
                expectsInput: true
            });
            return;
        }

        // Check for login question
        if (INIT_PATTERNS.login.test(this.buffer)) {
            this.emitQuestion({
                type: "login",
                prompt: "Log in to Claude Code?",
                options: ["yes", "no"],
                expectsInput: true
            });
            return;
        }
    }

    emitQuestion(question: InitQuestion) {
        globalStore.set(this.atoms.initStateAtom, {
            phase: "awaiting_response",
            question: question
        });
    }
}
```

**Challenge: Raw Output vs. NDJSON Stream**

Claude Code with `--output-format stream-json` may or may not send init prompts as NDJSON events. We need to handle both:

```typescript
// Option A: Questions come as NDJSON events
{
    "type": "input_request",
    "prompt": "Choose theme (light/dark):",
    "input_type": "selection"
}

// Option B: Questions come as raw STDERR/STDOUT
// (More likely for initialization prompts)
Choose theme (light/dark): _
```

**Solution:** Monitor both streams:
```typescript
// Monitor NDJSON stream (existing parseEvent)
parseEvent(event) {
    if (event.type === "input_request") {
        return handleInputRequest(event);
    }
}

// ALSO monitor raw PTY output via separate file
// Create new file: claude-code-raw.log
// Subscribe to it before NDJSON parsing
```

---

### Phase 3: Display Question UI

**New Component: InitializationPrompt.tsx**

```typescript
interface InitializationPromptProps {
    question: InitQuestion;
    onResponse: (answer: string) => void;
}

export const InitializationPrompt: React.FC<InitializationPromptProps> = ({
    question,
    onResponse
}) => {
    if (question.type === "theme") {
        return (
            <div className="init-prompt theme-prompt">
                <div className="prompt-text">{question.prompt}</div>
                <div className="prompt-options">
                    <button
                        className="option-btn light"
                        onClick={() => onResponse("light")}
                    >
                        ☀️ Light
                    </button>
                    <button
                        className="option-btn dark"
                        onClick={() => onResponse("dark")}
                    >
                        🌙 Dark
                    </button>
                </div>
            </div>
        );
    }

    if (question.type === "login") {
        return (
            <div className="init-prompt login-prompt">
                <div className="prompt-text">{question.prompt}</div>
                <div className="prompt-options">
                    <button
                        className="option-btn yes"
                        onClick={() => onResponse("y")}
                    >
                        ✓ Yes
                    </button>
                    <button
                        className="option-btn no"
                        onClick={() => onResponse("n")}
                    >
                        ✗ No
                    </button>
                </div>
            </div>
        );
    }

    // Fallback: Generic text input
    return (
        <div className="init-prompt generic-prompt">
            <div className="prompt-text">{question.prompt}</div>
            <input
                type="text"
                placeholder="Enter response..."
                onKeyDown={(e) => {
                    if (e.key === "Enter") {
                        onResponse(e.currentTarget.value);
                    }
                }}
            />
        </div>
    );
};
```

**Integration in agent-view.tsx:**

```typescript
const AgentViewInner = ({ ... }) => {
    const initState = useAtomValue(atoms.initStateAtom);

    return (
        <div className="agent-view">
            {/* Show init prompt if initializing */}
            {initState.phase === "awaiting_response" && (
                <InitializationPrompt
                    question={initState.question}
                    onResponse={(answer) => {
                        model.respondToInitPrompt(answer);
                    }}
                />
            )}

            {/* Regular document view */}
            {initState.phase === "ready" && (
                <DocumentView document={document} />
            )}
        </div>
    );
};
```

---

### Phase 4: Send Response

**ViewModel Method:**

```typescript
async respondToInitPrompt(answer: string) {
    console.log(`[agent] Responding to init prompt: ${answer}`);

    // 1. Send answer to Claude Code's STDIN
    const b64data = stringToBase64(answer + "\n");
    await RpcApi.ControllerInputCommand(TabRpcClient, {
        blockid: this.blockId,
        inputdata64: b64data
    });

    // 2. Update state to "processing"
    globalStore.set(this.atoms.initStateAtom, {
        phase: "processing",
        message: "Processing response..."
    });

    // 3. Continue monitoring for next question or completion
    // InitializationMonitor will detect next prompt or ready state
}
```

---

### Phase 5: Completion Detection

**How to know initialization is complete?**

```typescript
// Option A: Detect ready state from NDJSON
parseEvent(event) {
    if (event.type === "ready") {
        globalStore.set(this.atoms.initStateAtom, {
            phase: "ready"
        });
        this.initMonitor?.stop();
    }
}

// Option B: Timeout after last question
// If no new questions for 3 seconds, assume ready
const INIT_TIMEOUT = 3000;
setTimeout(() => {
    if (this.initState.phase === "processing") {
        this.completeInitialization();
    }
}, INIT_TIMEOUT);

// Option C: Detect first regular message
// When we get first content_block_start, init is done
parseEvent(event) {
    if (event.type === "message_start" && !this.initComplete) {
        this.completeInitialization();
    }
}
```

**Recommended: Hybrid approach**

```typescript
completeInitialization() {
    console.log("[agent] Initialization complete");

    globalStore.set(this.atoms.initStateAtom, {
        phase: "ready",
        message: "Connected to Claude Code"
    });

    // Update connection status
    globalStore.set(this.atoms.authAtom, {
        status: "connected"
    });

    // Stop monitoring for init questions
    this.initMonitor?.stop();

    // Start normal document parsing
    this.connectToTerminal();
}
```

---

## State Machine

```typescript
type InitPhase =
    | "disconnected"     // Initial state, show "Connect" button
    | "spawning"         // Process starting up
    | "awaiting_response" // Question displayed, waiting for user
    | "processing"       // Response sent, waiting for next question
    | "ready"            // Initialization complete, normal operation
    | "error";           // Something failed

interface InitState {
    phase: InitPhase;
    question?: InitQuestion;
    message?: string;
    error?: string;
}

// Transitions:
// disconnected --[Connect clicked]--> spawning
// spawning --[Process started]--> awaiting_response
// awaiting_response --[User responds]--> processing
// processing --[Next question detected]--> awaiting_response
// processing --[No more questions]--> ready
// any --[Error]--> error
```

---

## Data Capture: Raw PTY Output

**Problem:** NDJSON stream may not include initialization prompts

**Solution:** Capture raw PTY output separately

### Backend Changes (Go)

```go
// shellcontroller.go

// Create TWO output files:
// 1. claude-code.jsonl - NDJSON stream (existing)
// 2. claude-code-raw.log - Raw PTY output (new)

func (sc *ShellController) setupAndStartShellProcess(...) {
    // Create NDJSON file (existing)
    fsErr := filestore.WFS.MakeFile(ctx, sc.BlockId, "claude-code.jsonl", ...)

    // Create RAW output file (new)
    fsErr = filestore.WFS.MakeFile(ctx, sc.BlockId, "claude-code-raw.log", ...)

    // Tee PTY output to both files
    go func() {
        buf := make([]byte, 4096)
        for {
            n, err := ptyFile.Read(buf)
            if err != nil {
                break
            }

            // Write to BOTH files
            writeToFile(sc.BlockId, "claude-code.jsonl", buf[:n])
            writeToFile(sc.BlockId, "claude-code-raw.log", buf[:n])
        }
    }()
}
```

### Frontend Changes (TypeScript)

```typescript
// agent-model.ts

// Subscribe to BOTH streams
connectToTerminal() {
    // 1. Subscribe to NDJSON stream (existing)
    this.fileSubjectRef = getFileSubject(this.blockId, "claude-code.jsonl");
    this.fileSubjectSub = this.fileSubjectRef.subscribe((msg) => {
        this.handleTerminalData(msg);
    });

    // 2. Subscribe to raw output stream (new)
    this.rawSubjectRef = getFileSubject(this.blockId, "claude-code-raw.log");
    this.rawSubjectSub = this.rawSubjectRef.subscribe((msg) => {
        this.handleRawOutput(msg);
    });
}

handleRawOutput(msg: any) {
    if (msg.fileop === "append" && msg.data64) {
        const text = new TextDecoder().decode(base64ToArray(msg.data64));

        // Feed to initialization monitor
        this.initMonitor?.handleRawOutput(text);

        // Could also extract ANSI codes, control sequences, etc.
        this.extractControlData(text);
    }
}

extractControlData(text: string) {
    // Parse ANSI escape sequences
    // Example: \x1b[?1049h = Enter alternate screen buffer
    // Example: \x1b[2J = Clear screen

    // Look for Ctrl+O (ASCII 15) or other control chars
    if (text.includes('\x0F')) {
        console.log("[agent] Detected Ctrl+O");
    }
}
```

---

## Implementation Plan

### Phase 1: Minimal Viable POC (Day 1)

**Goal:** Prove basic question/answer relay works

**Scope:**
- Hardcode detection of ONE question (theme selection)
- Show simple UI prompt
- Send response back
- Log success

**Files to Create:**
```
frontend/app/view/agent/
├── components/
│   └── InitializationPrompt.tsx  (new)
└── init-monitor.ts               (new)
```

**Files to Modify:**
```
frontend/app/view/agent/
├── agent-model.ts         (add handleConnect, respondToInitPrompt)
├── agent-view.tsx         (add InitializationPrompt rendering)
├── state.ts               (add initStateAtom)
└── components/
    └── ConnectionStatus.tsx (add Connect button if not exists)
```

**Steps:**
1. Add `initStateAtom` to state.ts
2. Create `InitializationPrompt.tsx` with theme buttons
3. Add `InitializationMonitor` class in `init-monitor.ts`
4. Wire up `handleConnect()` in `agent-model.ts`
5. Render prompt in `agent-view.tsx`
6. Test manually: Click connect, see theme prompt, click Dark, verify response sent

**Success Criteria:**
- ✅ Click "Connect" spawns Claude Code
- ✅ Theme question appears in UI
- ✅ Clicking "Dark" sends "dark\n" to STDIN
- ✅ Console logs show round-trip worked

---

### Phase 2: Multi-Question Flow (Day 2)

**Goal:** Handle multiple sequential questions

**Scope:**
- Detect theme AND login questions
- Show prompts in sequence
- Transition to "ready" when done

**Steps:**
1. Add login pattern to `INIT_PATTERNS`
2. Add login case to `InitializationPrompt.tsx`
3. Implement completion detection
4. Test full flow: theme → login → ready

**Success Criteria:**
- ✅ Both questions appear in sequence
- ✅ Answers sent correctly
- ✅ State transitions to "ready"
- ✅ Document view appears after init

---

### Phase 3: Raw Output Capture (Day 3)

**Goal:** Capture raw PTY output for better detection

**Scope:**
- Backend: Tee output to claude-code-raw.log
- Frontend: Subscribe to raw file
- Use raw output for question detection

**Steps:**
1. Add raw file creation in `shellcontroller.go`
2. Tee PTY output to both files
3. Subscribe to raw file in `agent-model.ts`
4. Feed raw output to InitializationMonitor
5. Test detection accuracy improves

**Success Criteria:**
- ✅ Raw output file created
- ✅ Raw output subscription works
- ✅ Questions detected from raw output
- ✅ ANSI codes visible (optional: strip them)

---

### Phase 4: Polish & Edge Cases (Day 4)

**Goal:** Handle errors, timeouts, edge cases

**Scope:**
- Process startup failures
- Timeout if questions never appear
- Unexpected prompts
- Graceful error states

**Steps:**
1. Add error handling for process spawn failures
2. Add timeout for initialization (30 seconds max)
3. Add "Skip" button for unknown questions
4. Add retry logic

**Success Criteria:**
- ✅ Startup failures show error UI
- ✅ Timeout triggers fallback
- ✅ Unknown questions don't break UI
- ✅ User can retry failed initialization

---

## Testing Strategy

### Manual Testing

```
Test Case 1: Happy Path
1. Open agent widget
2. Click "Connect"
3. Verify theme question appears
4. Click "Dark"
5. Verify login question appears
6. Click "No"
7. Verify initialization completes
8. Verify document view shows ready state

Test Case 2: Different Choices
1. Click "Connect"
2. Click "Light" for theme
3. Click "Yes" for login
4. Verify initialization succeeds

Test Case 3: Process Failure
1. Stop backend (simulate crash)
2. Click "Connect"
3. Verify error state shown
4. Verify retry button works

Test Case 4: Timeout
1. Mock slow responses
2. Verify timeout after 30s
3. Verify error shown
```

### Automated Testing (Future)

```typescript
// init-monitor.test.ts
describe("InitializationMonitor", () => {
    it("detects theme question", () => {
        const monitor = new InitializationMonitor(mockAtoms);
        monitor.handleRawOutput("Choose theme (light/dark): ");

        const state = globalStore.get(mockAtoms.initStateAtom);
        expect(state.question.type).toBe("theme");
    });

    it("detects login question", () => {
        const monitor = new InitializationMonitor(mockAtoms);
        monitor.handleRawOutput("Log in? (y/n): ");

        const state = globalStore.get(mockAtoms.initStateAtom);
        expect(state.question.type).toBe("login");
    });
});
```

---

## Success Criteria

### POC Complete When:

- [x] User can click "Connect" and spawn fresh Claude Code instance
- [x] Theme question appears in custom UI (not terminal text)
- [x] User can click button to answer theme question
- [x] Response successfully reaches Claude Code's STDIN
- [x] Login question appears next
- [x] User can answer login question
- [x] Initialization completes and widget enters "ready" state
- [x] Normal document view appears after initialization
- [x] Raw PTY output captured for debugging/analysis

### Future Enhancements (Not in POC)

- [ ] Permission approval UI (similar pattern)
- [ ] Agent team messaging UI (similar pattern)
- [ ] Plan approval UI (similar pattern)
- [ ] Custom prompts for other interactive features
- [ ] Session resume handling
- [ ] Multiple agent coordination

---

## Open Questions

### 1. Does Claude Code initialization always prompt?

**Question:** Does `claude --output-format stream-json` ALWAYS show theme/login prompts on first run?

**Investigation Needed:**
- Test fresh install of Claude Code
- Test with existing config
- Check if `--skip-login` or similar flags exist

**Fallback:** If prompts don't appear consistently, we can trigger them manually or use this as a pattern for OTHER interactive flows.

---

### 2. NDJSON vs. Raw Output for Prompts

**Question:** Do initialization prompts come through NDJSON stream or only raw STDERR?

**Investigation Needed:**
- Capture actual Claude Code output during init
- Check if NDJSON includes `input_request` events
- Determine if we need raw output capture

**Decision:** Implement raw output capture anyway - useful for debugging and future features.

---

### 3. Fresh Environment Guarantee

**Question:** How do we ensure a truly fresh Claude Code instance?

**Options:**
1. `--clear-chat` flag (if exists)
2. Delete `~/.claude/` session files before spawn
3. Use separate workspace per widget (`--workspace my-agent`)
4. Accept cached state and work with it

**Recommendation:** Start with forcerestart, investigate flags later.

---

### 4. Control Sequences & Extra Data

**Question:** What other data can we extract from raw PTY output?

**Possibilities:**
- ANSI color codes (theme the UI accordingly)
- Cursor position commands (detect active regions)
- Alternate screen buffer (detect TUI mode)
- Bell characters (notification triggers)
- Progress bars / spinners (show in UI)

**Future Work:** Build an ANSI parser to extract semantic meaning from control sequences.

---

## File Structure

```
frontend/app/view/agent/
├── agent-model.ts                    (modified)
├── agent-view.tsx                    (modified)
├── state.ts                          (modified - add initStateAtom)
├── types.ts                          (modified - add InitQuestion)
├── init-monitor.ts                   (NEW)
├── components/
│   ├── InitializationPrompt.tsx      (NEW)
│   ├── ConnectionStatus.tsx          (modified - add Connect handler)
│   └── ...
└── agent-view.scss                   (modified - add init prompt styles)

pkg/blockcontroller/
└── shellcontroller.go                (modified - add raw output file)
```

---

## Next Steps After POC

### 1. Permission Approval UI
Use same pattern for tool permission prompts:
```typescript
{
    type: "permission",
    prompt: "Allow Bash: rm -rf node_modules?",
    options: ["Approve", "Reject"]
}
```

### 2. Agent Team Messaging UI
Use same pattern for incoming agent messages:
```typescript
{
    type: "agent_message",
    from: "backend-dev",
    content: "Found the bug in auth.ts",
    timestamp: Date.now()
}
```

### 3. Plan Approval UI
Use same pattern for plan approval:
```typescript
{
    type: "plan_approval",
    plan: "1. Fix auth bug\n2. Add tests\n3. Deploy",
    options: ["Approve", "Reject", "Request Changes"]
}
```

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Claude Code doesn't prompt on startup | POC fails | Use alternative test (manual prompt injection) |
| Questions don't match expected patterns | Detection fails | Add generic fallback prompt UI |
| Raw output not available | Can't detect prompts | Use NDJSON events only, document limitation |
| Process spawn takes too long | Poor UX | Add timeout, show progress spinner |
| STDIN relay doesn't work | Can't respond | Debug RpcApi, verify PTY setup |

---

## Validation Checklist

Before marking POC complete:

- [ ] Fresh Claude Code instance spawns on Connect
- [ ] At least ONE question detected and displayed
- [ ] User response successfully sent to STDIN
- [ ] Response acknowledged by Claude Code (next prompt or ready state)
- [ ] State machine transitions correctly
- [ ] Error states handled gracefully
- [ ] Code is documented and clean
- [ ] Demo video recorded showing full flow

---

## References

- **Agent Translation Layer:** `docs/AGENT_TRANSLATION_LAYER_ANALYSIS.md`
- **Unified Agent Spec:** `docs/SPEC_UNIFIED_AGENT_WIDGET.md`
- **Current Implementation:** `frontend/app/view/agent/`
- **I/O Relay:** `pkg/blockcontroller/shellcontroller.go`

---

## Appendix: Example Output Formats

### NDJSON Stream (Hypothetical)

```json
{"type":"stream_event","event":{"type":"init_prompt","prompt":"Choose theme (light/dark):","options":["light","dark"]}}
{"type":"stream_event","event":{"type":"init_response","value":"dark"}}
{"type":"stream_event","event":{"type":"init_prompt","prompt":"Log in? (y/n):","options":["y","n"]}}
{"type":"stream_event","event":{"type":"init_response","value":"n"}}
{"type":"stream_event","event":{"type":"ready"}}
```

### Raw PTY Output (Actual)

```
Choose theme (light/dark): dark
✓ Theme set to dark

Log in? (y/n): n
✓ Continuing without login

Ready to help! What would you like me to do?
```

### Atoms State Flow

```typescript
// Initial
{ phase: "disconnected" }

// After Connect clicked
{ phase: "spawning", message: "Starting Claude Code..." }

// Theme question detected
{
    phase: "awaiting_response",
    question: { type: "theme", prompt: "Choose theme", options: ["light", "dark"] }
}

// User clicked "Dark"
{ phase: "processing", message: "Processing response..." }

// Login question detected
{
    phase: "awaiting_response",
    question: { type: "login", prompt: "Log in?", options: ["yes", "no"] }
}

// User clicked "No"
{ phase: "processing", message: "Processing response..." }

// Init complete
{ phase: "ready", message: "Connected to Claude Code" }
```

---

## Conclusion

This POC proves the fundamental capability to:
1. Intercept Claude Code's interactive prompts
2. Present them in custom UI
3. Relay user responses back
4. Complete the bidirectional communication loop

**Success unlocks:**
- Permission approval UI
- Agent messaging UI
- Plan approval UI
- Any future interactive Claude Code features

**Estimated Effort:** 2-4 days for complete POC with polish

**Go/No-Go Decision:** After Phase 1 (1 day), we'll know if the approach works.
