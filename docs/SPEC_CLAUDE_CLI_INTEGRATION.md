# SPEC: Claude Code CLI Integration — AgentMux Agent Tab

> Written: 2026-02-20
> Verified against: Claude Code CLI v2.1.49, Windows 10, MSYS2/Git Bash
> All event formats captured from live CLI introspection (not docs)

---

## 1. Problem Statement

The agent tab currently has:
- A hardcoded wrong URL (`https://claude.ai/code/auth?redirect_uri=agentmux://auth`) that doesn't exist
- Wrong CLI flags (`--output-format stream-json` without `--verbose` — CLI rejects this)
- No auth state detection before spawning the CLI
- An attempt to detect auth URLs from stream-json output (impossible — `-p` mode has no interactive prompts)
- No understanding of the CLI's actual event format

---

## 2. Verified CLI Behavior

### 2.1 Auth Status Check

```bash
claude auth status --json
```

Returns:
```json
{
  "loggedIn": true,              // THE key field
  "authMethod": "claude.ai",    // "claude.ai" | "api-key" | null
  "apiProvider": "firstParty",  // "firstParty" | "anthropic" | null
  "email": "user@example.com",  // string | null
  "orgId": "uuid",              // string | null
  "orgName": null,              // string | null
  "subscriptionType": "max"     // "max" | "pro" | "teams" | "enterprise" | null
}
```

Exit code 0 regardless of login state. Parse `loggedIn` field.

### 2.2 Auth Login (Interactive)

```bash
claude auth login [--email user@example.com] [--sso]
```

- Opens system browser to `https://claude.ai/oauth/authorize` with PKCE flow
- Starts local HTTP callback server on dynamic port
- Waits for browser redirect, exchanges code for token
- Stores credentials in `~/.claude/.credentials.json`
- Prints text progress to stdout (NOT JSON)
- Exit code 0 on success, 1 on failure

### 2.3 Session Mode (What AgentMux Uses)

**Required flags:**
```bash
claude -p --verbose --output-format stream-json --include-partial-messages
```

| Flag | Required? | Why |
|------|-----------|-----|
| `-p` / `--print` | YES | Non-interactive mode |
| `--verbose` | YES | `stream-json` refuses to work without it |
| `--output-format stream-json` | YES | NDJSON streaming output |
| `--include-partial-messages` | YES | Token-by-token streaming (otherwise only complete messages) |
| `--input-format stream-json` | OPTIONAL | For multi-turn bidirectional streaming |
| `--dangerously-skip-permissions` | OPTIONAL | Skip tool permission prompts (sandboxed use) |

**Without `--verbose`:** CLI exits immediately with:
```
Error: When using --print, --output-format=stream-json requires --verbose
```

### 2.4 One-Shot vs Multi-Turn

| Mode | Command | Stdin | Behavior |
|------|---------|-------|----------|
| One-shot | `echo "prompt" \| claude -p --verbose --output-format stream-json --include-partial-messages` | Text prompt | Process prompt, stream events, emit result, exit |
| Multi-turn | `claude -p --verbose --output-format stream-json --include-partial-messages --input-format stream-json` | JSON messages | Keep session alive, accept multiple inputs |
| Resume | `claude -p --verbose --output-format stream-json --include-partial-messages --resume <session_id>` | Text prompt | Continue previous conversation |

### 2.5 MSYS2/Windows Limitation

Named pipes (FIFOs) do not work reliably on MSYS2/Git Bash. For persistent bidirectional sessions, the Rust shell controller's PTY is the correct IPC mechanism (already in place via `ControllerInputCommand` / `getFileSubject`).

---

## 3. Complete Event Sequence (Verified)

Captured from: `echo "say hello" | claude -p --verbose --output-format stream-json --include-partial-messages`

### 3.1 Event Flow

```
LINE 1:  system.init          ← always first, confirms auth OK
LINE 2:  stream_event         ← message_start
LINE 3:  stream_event         ← content_block_start
LINE 4+: stream_event         ← content_block_delta (×N, token-by-token)
LINE N:  assistant             ← complete message (arrives mid-stream!)
LINE N+1: stream_event        ← content_block_stop
LINE N+2: stream_event        ← message_delta (has stop_reason)
LINE N+3: stream_event        ← message_stop
LINE N+4: rate_limit_event    ← optional, rate limit info
LINE LAST: result              ← always last, has cost/usage/error info
```

### 3.2 Event Type Reference

#### `system` (init) — Line 1, Always Present

```json
{
  "type": "system",
  "subtype": "init",
  "cwd": "C:\\Systems\\agentmux",
  "session_id": "b3e5aeb5-5511-4768-b3d9-3bfbad144c47",
  "tools": ["Task", "Bash", "Glob", "Grep", "Read", "Edit", "Write", ...],
  "mcp_servers": [{"name": "agentmux", "status": "connected"}],
  "model": "claude-opus-4-6",
  "permissionMode": "default",
  "apiKeySource": "none",
  "claude_code_version": "2.1.49",
  "agents": ["Bash", "general-purpose", "Explore", "Plan", ...],
  "skills": ["keybindings-help", "debug"],
  "plugins": [],
  "fast_mode_state": "off",
  "uuid": "58499f64-c7b2-4702-b27d-dfd2be92c084"
}
```

**Key insight:** Receiving `system.init` = auth succeeded, session is live.

#### `stream_event` — Wraps Raw API Events

```json
{
  "type": "stream_event",
  "event": {
    "type": "content_block_delta",
    "index": 0,
    "delta": {"type": "text_delta", "text": "hello"}
  },
  "session_id": "uuid",
  "parent_tool_use_id": null,
  "uuid": "uuid"
}
```

Inner `event.type` values:
- `message_start` — new message beginning
- `content_block_start` — new content block (text, tool_use, thinking)
- `content_block_delta` — incremental content (`text_delta`, `input_json_delta`, `thinking_delta`)
- `content_block_stop` — content block finished
- `message_delta` — message metadata update (`stop_reason`, usage)
- `message_stop` — message complete

#### `assistant` — Complete Message

```json
{
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-6",
    "id": "msg_xxx",
    "role": "assistant",
    "content": [
      {"type": "text", "text": "STATE_MACHINE_TEST_OK"}
    ],
    "stop_reason": null,
    "usage": {
      "input_tokens": 2,
      "cache_creation_input_tokens": 3845,
      "cache_read_input_tokens": 20694,
      "output_tokens": 11,
      "service_tier": "standard"
    }
  },
  "parent_tool_use_id": null,
  "session_id": "uuid"
}
```

Content block types in `message.content[]`:
- `{"type": "text", "text": "..."}` — text output
- `{"type": "tool_use", "id": "toolu_xxx", "name": "Read", "input": {...}}` — tool call
- `{"type": "thinking", "thinking": "..."}` — extended thinking

**NOTE:** The `assistant` event arrives BEFORE `content_block_stop` / `message_delta` / `message_stop`. This is different from what you might expect — the complete message is emitted while stream events are still flowing.

#### `user` — Tool Results

```json
{
  "type": "user",
  "message": {
    "content": [
      {
        "type": "tool_result",
        "tool_use_id": "toolu_xxx",
        "content": "file contents here..."
      }
    ]
  },
  "session_id": "uuid"
}
```

#### `rate_limit_event` — Rate Limit Status

```json
{
  "type": "rate_limit_event",
  "rate_limit_info": {
    "status": "allowed",
    "resetsAt": 1771632000,
    "rateLimitType": "five_hour",
    "overageStatus": "rejected",
    "overageDisabledReason": "org_level_disabled_until",
    "isUsingOverage": false
  }
}
```

#### `result` — Last Event, Always Present

```json
{
  "type": "result",
  "subtype": "success",
  "is_error": false,
  "duration_ms": 2094,
  "duration_api_ms": 1811,
  "num_turns": 1,
  "result": "STATE_MACHINE_TEST_OK",
  "stop_reason": null,
  "session_id": "uuid",
  "total_cost_usd": 0.03466325,
  "usage": {
    "input_tokens": 2,
    "cache_creation_input_tokens": 3845,
    "cache_read_input_tokens": 20694,
    "output_tokens": 11,
    "server_tool_use": {"web_search_requests": 0, "web_fetch_requests": 0},
    "service_tier": "standard"
  },
  "modelUsage": {
    "claude-opus-4-6": {
      "inputTokens": 2,
      "outputTokens": 11,
      "cacheReadInputTokens": 20694,
      "cacheCreationInputTokens": 3845,
      "costUSD": 0.03466325,
      "contextWindow": 200000,
      "maxOutputTokens": 32000
    }
  },
  "permission_denials": []
}
```

**Cost field:** `total_cost_usd` (NOT `cost_usd` as previously documented)

---

## 4. State Machine for AgentMux

### 4.1 Top-Level States

```
┌─────────────────────────────────────────────────────────────────┐
│                     AGENT TAB LIFECYCLE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  SETUP_PENDING ──→ CHECKING_AUTH ──→ AUTH_OK ──→ SESSION_ACTIVE │
│       │                  │                            │          │
│       │                  ▼                            ▼          │
│       │           AUTH_REQUIRED ──→ LOGGING_IN   SESSION_ERROR  │
│       │                  │              │             │          │
│       │                  │              ▼             │          │
│       │                  │         LOGIN_DONE ────────┘          │
│       │                  │              │                        │
│       │                  └──────────────┘                        │
│       │                                                          │
│       └──→ SETUP_WIZARD (first time only)                       │
│                  │                                               │
│                  └──→ CHECKING_AUTH                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 State Definitions

| State | Description | UI Shows | Trigger to Next |
|-------|-------------|----------|-----------------|
| `SETUP_PENDING` | Loading provider config from store | Loading spinner | Config loaded |
| `SETUP_WIZARD` | No provider configured yet | SetupWizard component | User completes wizard |
| `CHECKING_AUTH` | Running `claude auth status --json` | "Checking authentication..." | Auth status result |
| `AUTH_OK` | `loggedIn: true` confirmed | Transition immediately | Spawn CLI session |
| `AUTH_REQUIRED` | `loggedIn: false` | "Authentication required" + Login button | User clicks Login |
| `LOGGING_IN` | Running `claude auth login` in PTY | "Complete login in browser..." + Cancel button | Process exits |
| `LOGIN_DONE` | `claude auth login` exited 0 | Transition immediately | Re-check auth |
| `LOGIN_FAILED` | `claude auth login` exited non-0 | Error message + Retry button | User clicks Retry |
| `SESSION_STARTING` | CLI spawned, waiting for `system.init` | "Starting session..." | `system.init` received |
| `SESSION_ACTIVE` | `system.init` received, streaming | Agent document + footer input | User interaction / result event |
| `SESSION_ERROR` | `result.is_error: true` or process crash | Error message + Restart button | User clicks Restart |
| `SESSION_COMPLETE` | `result` event received (one-shot mode) | Result display + "New prompt" | User sends new prompt |
| `RATE_LIMITED` | `rate_limit_event` with `status: "limited"` | "Rate limited, resets at..." | Timer / retry |

### 4.3 State Transitions

```
SETUP_PENDING
  ├─ [config.setup_complete = false] → SETUP_WIZARD
  └─ [config.setup_complete = true]  → CHECKING_AUTH

SETUP_WIZARD
  └─ [user completes wizard] → CHECKING_AUTH

CHECKING_AUTH
  ├─ Run: claude auth status --json
  ├─ Parse JSON: check loggedIn field
  ├─ [loggedIn = true]  → AUTH_OK
  ├─ [loggedIn = false] → AUTH_REQUIRED
  └─ [command fails]    → AUTH_REQUIRED (assume not logged in)

AUTH_OK
  └─ [immediate] → SESSION_STARTING

AUTH_REQUIRED
  ├─ UI: "Not authenticated" + "Login" button
  ├─ [user clicks Login] → LOGGING_IN
  └─ [user enters API key] → set ANTHROPIC_API_KEY → CHECKING_AUTH

LOGGING_IN
  ├─ Spawn: claude auth login (in PTY, so user sees browser prompt)
  ├─ [process exits 0]     → LOGIN_DONE
  ├─ [process exits non-0] → LOGIN_FAILED
  └─ [user clicks Cancel]  → SIGINT → AUTH_REQUIRED

LOGIN_DONE
  └─ [immediate] → CHECKING_AUTH (re-verify)

LOGIN_FAILED
  ├─ UI: "Login failed" + error + Retry button
  └─ [user clicks Retry] → LOGGING_IN

SESSION_STARTING
  ├─ Set block meta: cmd="claude", cmd:args=["-p","--verbose","--output-format","stream-json","--include-partial-messages"]
  ├─ ControllerResyncCommand (force restart)
  ├─ Wait for first NDJSON line from terminal output
  ├─ [type = "system", subtype = "init"] → SESSION_ACTIVE
  ├─ [type = "result", is_error = true]  → SESSION_ERROR
  ├─ [non-JSON text on stderr]           → SESSION_ERROR
  └─ [timeout 30s, no output]            → SESSION_ERROR

SESSION_ACTIVE
  ├─ Process stream_event → render tokens in document
  ├─ Process assistant → render complete message
  ├─ Process user → render tool results
  ├─ [type = "result", is_error = false] → SESSION_COMPLETE
  ├─ [type = "result", is_error = true]  → SESSION_ERROR
  ├─ [type = "rate_limit_event", status = "limited"] → RATE_LIMITED
  ├─ [process exits unexpectedly]        → SESSION_ERROR
  └─ [user sends message via footer]     → write to stdin (ControllerInputCommand)

SESSION_COMPLETE
  ├─ Display cost from result.total_cost_usd
  ├─ [user sends new prompt] → SESSION_STARTING (new -p invocation or --resume)
  └─ [user closes tab]       → dispose

SESSION_ERROR
  ├─ Display error from result or stderr
  ├─ [user clicks Restart] → CHECKING_AUTH
  └─ [user closes tab]     → dispose

RATE_LIMITED
  ├─ Display: "Rate limited until {resetsAt}"
  ├─ [timer expires] → SESSION_ACTIVE (continue processing)
  └─ [user clicks Retry] → SESSION_STARTING
```

### 4.4 Rust Command for Auth Check

New command needed in `src-tauri/src/commands/providers.rs`:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CliAuthStatus {
    #[serde(rename = "loggedIn")]
    pub logged_in: bool,
    #[serde(rename = "authMethod")]
    pub auth_method: Option<String>,
    #[serde(rename = "apiProvider")]
    pub api_provider: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "subscriptionType")]
    pub subscription_type: Option<String>,
}

#[tauri::command]
pub async fn check_cli_auth_status(provider: String) -> Result<CliAuthStatus, String> {
    // Run: claude auth status --json (or gemini/codex equivalent)
    // Parse JSON output
    // Return structured result
}
```

### 4.5 Auth Login via PTY

For `claude auth login`, we spawn it as a regular shell command in the agent tab's PTY. The user sees the CLI output ("Opening browser..."), completes auth in browser, and the CLI exits. AgentMux detects exit code 0 and transitions to `CHECKING_AUTH`.

This means the `cmd` block meta switches between two modes:
1. **Login mode:** `cmd="claude"`, `cmd:args=["auth", "login"]`
2. **Session mode:** `cmd="claude"`, `cmd:args=["-p", "--verbose", "--output-format", "stream-json", "--include-partial-messages"]`

---

## 5. What Needs to Change

### 5.1 Provider Registry — Fix Default Args

**File:** `frontend/app/view/agent/providers/index.ts`

```typescript
// WRONG (current):
defaultArgs: ["--output-format", "stream-json"]

// CORRECT:
defaultArgs: ["-p", "--verbose", "--output-format", "stream-json", "--include-partial-messages"]
```

### 5.2 Agent Model — Auth State Machine

**File:** `frontend/app/view/agent/agent-model.ts`

Replace the current `initializeProvider()` with the state machine:

1. Load provider config from store
2. If `setup_complete`: run `check_cli_auth_status("claude")`
3. Based on `loggedIn`:
   - `true` → set block meta to session mode args, start CLI, wait for `system.init`
   - `false` → set auth state to `AUTH_REQUIRED`, show login UI
4. Login UI triggers `claude auth login` in PTY
5. On exit code 0 → re-check auth → start session

### 5.3 Translator — Fix Event Mapping

**File:** `frontend/app/view/agent/providers/claude-translator.ts`

The translator currently expects the OLD format (inner `stream_event.event` wrappers from the old code). It needs to handle the ACTUAL format:

- Top-level `type` field: `system`, `stream_event`, `assistant`, `user`, `rate_limit_event`, `result`
- `stream_event.event` contains raw Anthropic API events
- `assistant.message.content[]` has the complete message (text, tool_use, thinking blocks)
- `user.message.content[]` has tool_result blocks
- `result` has cost, error status, session info

### 5.4 ConnectionStatus — Auth States

**File:** `frontend/app/view/agent/components/ConnectionStatus.tsx`

States to render:
- `CHECKING_AUTH` → spinner + "Checking authentication..."
- `AUTH_REQUIRED` → "Not authenticated" + Login button + optional API key input
- `LOGGING_IN` → "Complete login in your browser..." + Cancel button
- `LOGIN_FAILED` → Error message + Retry button

### 5.5 Rust Backend — New Command

**File:** `src-tauri/src/commands/providers.rs`

Add `check_cli_auth_status(provider)` command that runs the CLI's auth check and returns structured result.

### 5.6 Remove Dead Code

- Remove `open_claude_code_auth` from `claudecode.rs` (hardcoded wrong URL)
- Remove `handle_auth_callback` from `claudecode.rs` (no deep link OAuth flow)
- Remove `claude-code-auth-started/success/error` event listeners
- Remove `checkForAuthUrlInText` / `checkForAuthUrlInEvent` from agent model (URLs won't appear in stream output)

---

## 6. Multi-Turn Session Strategy

### Option A: Repeated `-p --resume` (Simpler)

Each user message is a new `claude -p --resume <session_id>` invocation:
- Session continuity via `--resume`
- Clean process lifecycle (start → stream → result → exit)
- No need for bidirectional streaming
- Higher startup overhead per message (~1-2s)

### Option B: `--input-format stream-json` (Lower Latency)

Single long-lived process with bidirectional JSON:
- Input: `{"type":"user_input","content":"..."}` written to stdin
- Output: continuous NDJSON stream on stdout
- Lower latency (no process restart per message)
- Requires robust stdin pipe management
- MSYS2 FIFO limitation is a non-issue since Rust PTY handles it natively

### Recommendation

Start with **Option A** (repeated `--resume`). It's simpler, works with the existing shell controller, and the `session_id` from `system.init` / `result` events gives us continuity. Switch to Option B later if latency is a problem.

---

## 7. Implementation Order

| Phase | What | Files |
|-------|------|-------|
| 1 | Fix provider defaultArgs, add `--verbose` | `providers/index.ts` |
| 2 | Add `check_cli_auth_status` Rust command | `providers.rs`, `mod.rs`, `lib.rs`, `custom.d.ts`, `tauri-api.ts` |
| 3 | Rewrite agent model init as state machine | `agent-model.ts`, `state.ts` |
| 4 | Rewrite translator for actual event format | `claude-translator.ts` |
| 5 | Update ConnectionStatus for auth states | `ConnectionStatus.tsx` |
| 6 | Remove dead OAuth code | `claudecode.rs`, `agent-model.ts` |
| 7 | Test: not-logged-in flow | Manual test |
| 8 | Test: logged-in flow | Manual test |
