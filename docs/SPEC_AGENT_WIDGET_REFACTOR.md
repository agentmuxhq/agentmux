# Agent Widget Refactor - Claude Code Integration

**Date:** 2026-02-16
**Status:** Draft Specification
**Author:** AgentA

---

## Executive Summary

Refactor the Agent widget to use a **Claude Code-style input interface** instead of the current chat-style UI. The goal is to create a minimal, focused interface that mirrors Claude Code's input experience while integrating Claude Code authentication and API connectivity.

**Key Changes:**
- Remove chat-style send button and footer action buttons
- Implement Claude Code-style input (no send button, Enter to submit)
- Add single "Connect" button for Claude Code authentication
- Integrate Claude Code API for agent communication
- Streamline UI to be minimalist and functional

---

## 1. Current State Analysis

### 1.1 Current Components

**AgentFooter** (`frontend/app/view/agent/components/AgentFooter.tsx`):
- **Input area:** Textarea with "Send" button (chat-style)
- **Action buttons:**
  - Expand All
  - Collapse All
  - Export MD
  - Export HTML
  - Clear

**AgentViewModel** (`frontend/app/view/agent/agent-model.ts`):
- Connects to terminal via `claude-code.jsonl` file subject
- Uses NDJSON stream parser (`ClaudeCodeStreamParser`)
- Sends input via `ControllerInputCommand` (pipes to stdin)
- Currently designed for local process communication, NOT API-based

### 1.2 Current Flow

```
User Input → Textarea → "Send" Button → ControllerInputCommand → Terminal stdin → Local Process
                                                                                      ↓
                                     ← Terminal file subject ← NDJSON stream ← stdout
```

### 1.3 Problems with Current Design

1. **Chat-style UI doesn't match Claude Code UX**
   - "Send" button creates friction
   - Multiple action buttons clutter the interface
   - Doesn't feel like a code assistant, feels like a chat app

2. **No Claude Code API integration**
   - Currently uses local process stdio
   - No authentication mechanism
   - Can't connect to Claude Code service

3. **Button overload**
   - 6 buttons in footer (Send + 5 actions)
   - Most actions are rarely used
   - UI feels heavy, not minimal

4. **No connection status**
   - User doesn't know if connected to Claude Code
   - No way to authenticate
   - No visual feedback for connection state

---

## 2. New Design Goals

### 2.1 Design Principles

1. **Minimal UI** - Only show what's necessary
2. **Claude Code-style** - Match the input experience users expect
3. **Connection-first** - Make authentication/connection explicit and clear
4. **Progressive disclosure** - Show advanced features only when needed

### 2.2 Visual Design Reference

**Claude Code Input Characteristics:**
```
┌─────────────────────────────────────────────────┐
│  [Your input text here...]                      │
│                                                  │
│  Enter to send • Shift+Enter for newline        │
└─────────────────────────────────────────────────┘
```

**Key features:**
- No visible "Send" button
- Subtle hint text for shortcuts
- Clean, minimal border
- Auto-expanding textarea
- Enter = send, Shift+Enter = newline

---

## 3. UI/UX Changes

### 3.1 Footer Redesign

**Remove:**
- ❌ "Send" button (use Enter key instead)
- ❌ "Expand All" button
- ❌ "Collapse All" button
- ❌ "Export MD" button
- ❌ "Export HTML" button
- ❌ "Clear" button

**Keep:**
- ✅ Textarea input (redesigned)

**Add:**
- ✅ Keyboard shortcut hints (subtle, below textarea)
- ✅ Connection status indicator (when connected)

### 3.2 New Connection UI

**When NOT connected to Claude Code:**

```
┌─────────────────────────────────────────────────┐
│  🔌 Not Connected to Claude Code                │
│                                                  │
│  [Connect to Claude Code]  ← Button             │
│                                                  │
│  Connect your Claude Code account to use        │
│  AI-powered assistance.                         │
└─────────────────────────────────────────────────┘
```

**When connected:**

```
┌─────────────────────────────────────────────────┐
│  What would you like me to help with?           │
│                                                  │
│  Enter to send • Shift+Enter for newline        │
│  ✓ Connected to Claude Code                     │
└─────────────────────────────────────────────────┘
```

### 3.3 Input Behavior

**Keyboard shortcuts:**
- **Enter** - Send message (like Claude Code)
- **Shift+Enter** - New line
- **Cmd/Ctrl+K** - Clear input
- **Cmd/Ctrl+/** - Show help overlay (future)

**Auto-resize:**
- Start at 2 rows
- Expand to max 10 rows as user types
- Scroll beyond that

---

## 4. Claude Code Authentication

### 4.1 Authentication Flow

```
┌─────────────┐      ┌──────────────┐      ┌─────────────┐
│   User      │      │  AgentMux    │      │ Claude Code │
│  (Widget)   │      │   (Tauri)    │      │   (API)     │
└──────┬──────┘      └──────┬───────┘      └──────�┬──────┘
       │                    │                      │
       │  1. Click          │                      │
       │  "Connect"         │                      │
       ├───────────────────>│                      │
       │                    │                      │
       │                    │  2. Open browser     │
       │                    │     (auth URL)       │
       │                    ├─────────────────────>│
       │                    │                      │
       │                    │  3. User logs in     │
       │                    │     via browser      │
       │                    │                      │
       │                    │  4. Redirect with    │
       │                    │     auth code        │
       │                    │<─────────────────────│
       │                    │                      │
       │                    │  5. Exchange code    │
       │                    │     for token        │
       │                    ├─────────────────────>│
       │                    │                      │
       │                    │  6. Return token     │
       │                    │<─────────────────────│
       │  7. Update UI      │                      │
       │  "Connected"       │                      │
       │<───────────────────│                      │
       │                    │                      │
```

### 4.2 Implementation Details

**Step 1: Connect Button Click**
- Frontend component: `<ConnectButton />` in `AgentFooter`
- Calls: `getApi().openClaudeCodeAuth()`

**Step 2-4: Browser OAuth Flow**
- Tauri command: `open_claude_code_auth`
- Opens system browser to: `https://claude.ai/code/auth?redirect_uri=agentmux://auth`
- User logs in via browser
- Claude Code redirects to: `agentmux://auth?code=ABC123`
- Tauri deep link handler captures the code

**Step 5-6: Token Exchange**
- Backend RPC: `ExchangeClaudeCodeToken(code)`
- Exchanges authorization code for access token
- Stores token securely in AgentMux config

**Step 7: Update UI**
- Frontend receives auth success event
- Updates connection state atom
- Shows "Connected" indicator
- Enables input field

### 4.3 Token Storage

**Location:** `~/.config/com.a5af.agentmux/claude-code-auth.json`

```json
{
  "access_token": "sk-ant-...",
  "refresh_token": "...",
  "expires_at": 1708128000,
  "user_email": "user@example.com"
}
```

**Security:**
- File permissions: `0600` (user read/write only)
- Tokens encrypted at rest (future enhancement)
- Never logged or exposed in UI

---

## 5. Claude Code API Integration

### 5.1 Current Architecture (Local Process)

```
AgentViewModel
    ↓
ControllerInputCommand (stdin)
    ↓
Local Process (claude --output-format stream-json)
    ↓
Terminal file subject (stdout → claude-code.jsonl)
    ↓
ClaudeCodeStreamParser
    ↓
Document atoms (rendered in UI)
```

### 5.2 New Architecture (API-based)

```
AgentViewModel
    ↓
ClaudeCodeApiClient.sendMessage(text, conversationId)
    ↓
POST https://api.anthropic.com/v1/messages (streaming)
    ↓
Stream parser (SSE → NDJSON events)
    ↓
Document atoms (rendered in UI)
```

### 5.3 Hybrid Approach (Backwards Compatible)

**Support both modes:**

1. **Local Mode** (current)
   - Uses `claude` CLI process
   - No auth required
   - Works offline

2. **API Mode** (new)
   - Uses Claude Code API
   - Requires authentication
   - Cloud-based, always up-to-date

**Detection:**
```typescript
if (hasClaudeCodeAuth()) {
    // Use API mode
    this.client = new ClaudeCodeApiClient(token);
} else {
    // Use local mode (fallback)
    this.connectToTerminal();
}
```

---

## 6. Implementation Plan

### Phase 1: UI Refactor (Minimal Input)

**Goal:** Remove buttons, implement Claude Code-style input

**Files to modify:**
- `frontend/app/view/agent/components/AgentFooter.tsx`
  - Remove all action buttons
  - Remove "Send" button
  - Add keyboard shortcuts (Enter, Shift+Enter)
  - Add subtle hint text
  - Implement auto-resize textarea

**Tasks:**
- [ ] Remove `<button>` elements for Send, Expand All, etc.
- [ ] Update `handleKeyDown` to send on Enter (not Shift+Enter)
- [ ] Add hint text component below textarea
- [ ] Style textarea to match Claude Code aesthetic
- [ ] Test keyboard shortcuts

**Acceptance Criteria:**
- Enter key sends message
- Shift+Enter creates new line
- No visible send button
- UI looks clean and minimal

---

### Phase 2: Connection UI

**Goal:** Add "Connect to Claude Code" button and connection state

**Files to create:**
- `frontend/app/view/agent/components/ConnectionStatus.tsx`
  - Shows "Not Connected" state with Connect button
  - Shows "Connected" state with email/status
  - Handles connect button click

**Files to modify:**
- `frontend/app/view/agent/agent-view.tsx`
  - Conditionally show `ConnectionStatus` or input based on auth state
- `frontend/app/view/agent/state.ts`
  - Add `authStateAtom` (connected | disconnected)
  - Add `userInfoAtom` (email, etc.)

**Tasks:**
- [ ] Create `ConnectionStatus` component
- [ ] Add auth state atoms
- [ ] Implement `getApi().openClaudeCodeAuth()` Tauri command
- [ ] Add deep link handler for `agentmux://auth`
- [ ] Wire up connect button to auth flow

**Acceptance Criteria:**
- "Connect" button opens browser
- After auth, UI updates to "Connected"
- User email displayed when connected

---

### Phase 3: Claude Code Authentication (Backend)

**Goal:** Implement OAuth flow and token storage

**Files to create:**
- `cmd/server/pkg/claudecode/auth.go`
  - OAuth flow handlers
  - Token exchange
  - Token storage/retrieval
- `src-tauri/src/commands/claudecode.rs`
  - Tauri command: `open_claude_code_auth`
  - Deep link handler: `agentmux://auth`

**Files to modify:**
- `src-tauri/src/lib.rs`
  - Register deep link protocol
  - Register Tauri commands

**Tasks:**
- [ ] Implement OAuth flow (authorization code grant)
- [ ] Create token exchange endpoint
- [ ] Implement secure token storage
- [ ] Add token refresh logic
- [ ] Add Tauri deep link handler

**Acceptance Criteria:**
- Browser opens to Claude Code auth page
- After login, app receives auth code
- Token stored securely
- Token can be retrieved for API calls

---

### Phase 4: Claude Code API Client

**Goal:** Implement API-based communication with Claude Code

**Files to create:**
- `frontend/app/view/agent/api-client.ts`
  - `ClaudeCodeApiClient` class
  - Methods: `sendMessage`, `streamResponse`
  - SSE stream parsing

**Files to modify:**
- `frontend/app/view/agent/agent-model.ts`
  - Add mode detection (local vs API)
  - Use `ClaudeCodeApiClient` when authenticated
  - Fallback to terminal mode when not authenticated

**Tasks:**
- [ ] Create `ClaudeCodeApiClient` class
- [ ] Implement `POST /v1/messages` with streaming
- [ ] Parse SSE events into NDJSON format
- [ ] Handle rate limits and errors
- [ ] Add conversation persistence

**Acceptance Criteria:**
- Messages sent via API when connected
- Responses streamed in real-time
- Errors handled gracefully
- Falls back to local mode if not connected

---

### Phase 5: Polish & Testing

**Goal:** Final touches, error handling, and testing

**Tasks:**
- [ ] Add loading states during auth
- [ ] Add error messages for auth failures
- [ ] Add "Disconnect" option in settings
- [ ] Add connection retry logic
- [ ] Test with expired tokens
- [ ] Test with no internet connection
- [ ] Add telemetry for auth success/failure rates

**Acceptance Criteria:**
- All error cases handled
- Loading states smooth
- Disconnection works correctly
- Tests pass

---

## 7. API Reference

### 7.1 Tauri Commands

**`open_claude_code_auth()`**
```typescript
invoke('open_claude_code_auth'): Promise<void>
```
Opens system browser to Claude Code auth URL.

**`get_claude_code_auth()`**
```typescript
invoke('get_claude_code_auth'): Promise<{
  connected: boolean;
  email?: string;
  expires_at?: number;
}>
```
Returns current auth status.

**`disconnect_claude_code()`**
```typescript
invoke('disconnect_claude_code'): Promise<void>
```
Clears stored auth token.

### 7.2 Backend RPC

**`ExchangeClaudeCodeToken(code: string)`**
```go
func (s *BackendService) ExchangeClaudeCodeToken(code string) (*AuthToken, error)
```
Exchanges authorization code for access token.

**`GetClaudeCodeAuth()`**
```go
func (s *BackendService) GetClaudeCodeAuth() (*AuthInfo, error)
```
Returns current auth status.

---

## 8. File Structure

```
frontend/app/view/agent/
├── components/
│   ├── AgentFooter.tsx          # ← REFACTOR (remove buttons, simplify input)
│   ├── ConnectionStatus.tsx     # ← NEW (Connect button, status display)
│   ├── AgentHeader.tsx
│   ├── MarkdownBlock.tsx
│   ├── ToolBlock.tsx
│   └── ...
├── agent-model.ts                # ← MODIFY (add API mode detection)
├── agent-view.tsx                # ← MODIFY (conditionally show connection UI)
├── api-client.ts                 # ← NEW (Claude Code API client)
├── state.ts                      # ← MODIFY (add auth atoms)
└── stream-parser.ts

src-tauri/src/
├── commands/
│   ├── claudecode.rs             # ← NEW (auth commands)
│   └── mod.rs
└── lib.rs                        # ← MODIFY (register deep links)

cmd/server/pkg/
└── claudecode/
    ├── auth.go                   # ← NEW (OAuth flow)
    ├── client.go                 # ← NEW (API client)
    └── storage.go                # ← NEW (token storage)
```

---

## 9. UI Mockups

### 9.1 Disconnected State

```
┌───────────────────────────────────────────────────────────┐
│ Agent                                                  [X] │
├───────────────────────────────────────────────────────────┤
│                                                           │
│                      🔌                                   │
│                                                           │
│           Not Connected to Claude Code                   │
│                                                           │
│  Claude Code integration lets you use AI-powered         │
│  assistance directly in AgentMux.                        │
│                                                           │
│         ┌─────────────────────────────┐                  │
│         │  Connect to Claude Code     │                  │
│         └─────────────────────────────┘                  │
│                                                           │
│  By connecting, you agree to Claude's Terms of Service   │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

### 9.2 Connected State (Empty Document)

```
┌───────────────────────────────────────────────────────────┐
│ Agent                                  ✓ user@example.com │
├───────────────────────────────────────────────────────────┤
│                                                           │
│                      🤖                                   │
│                                                           │
│           Ready to assist                                │
│                                                           │
│  Ask me anything about your code, architecture, or       │
│  debugging. I have access to your workspace context.     │
│                                                           │
├───────────────────────────────────────────────────────────┤
│  What would you like me to help with?                    │
│  ┌─────────────────────────────────────────────────────┐ │
│  │                                                     │ │
│  └─────────────────────────────────────────────────────┘ │
│  Enter to send • Shift+Enter for newline               │
└───────────────────────────────────────────────────────────┘
```

### 9.3 Connected State (Active Conversation)

```
┌───────────────────────────────────────────────────────────┐
│ Agent                                  ✓ user@example.com │
├───────────────────────────────────────────────────────────┤
│  👤 You                                                   │
│  How do I optimize this React component?                 │
│                                                           │
│  🤖 Claude                                                │
│  I can help you optimize this component. Here are some   │
│  suggestions:                                            │
│                                                           │
│  1. Use React.memo() to prevent unnecessary re-renders   │
│  2. Extract expensive calculations to useMemo()          │
│  3. Use useCallback for event handlers                   │
│                                                           │
│  🔧 Tool: read_file                                      │
│  ├─ src/components/MyComponent.tsx                       │
│  └─ [1.2 KB] ✓                                           │
│                                                           │
├───────────────────────────────────────────────────────────┤
│  What would you like me to help with?                    │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ Show me an example                                  │ │
│  └─────────────────────────────────────────────────────┘ │
│  Enter to send • Shift+Enter for newline               │
└───────────────────────────────────────────────────────────┘
```

---

## 10. Success Metrics

**User Experience:**
- ✅ Input feels natural (like Claude Code)
- ✅ Connection is clear and obvious
- ✅ No confusion about authentication state

**Technical:**
- ✅ Auth flow completes in < 30 seconds
- ✅ API responses stream in real-time
- ✅ Token refresh happens transparently
- ✅ Falls back gracefully when offline

**Adoption:**
- 📊 % of users who connect to Claude Code
- 📊 % of messages sent via API vs local mode
- 📊 Auth success rate
- 📊 Average session length

---

## 11. Open Questions

1. **Conversation persistence**
   - Should we save conversation history locally?
   - How long should we retain conversations?

2. **Multi-account support**
   - Do we support multiple Claude Code accounts?
   - How do users switch between them?

3. **Offline mode**
   - What happens when user loses internet during conversation?
   - Should we queue messages and send when reconnected?

4. **Rate limiting**
   - How do we handle Claude API rate limits?
   - Do we show quota/usage to user?

5. **Local mode deprecation**
   - Do we eventually remove local `claude` CLI mode?
   - Or keep it as a fallback forever?

---

## 12. Future Enhancements

**Phase 6+ (Post-MVP):**
- [ ] Context menu for actions (Expand All, Export, etc.)
- [ ] Keyboard shortcut overlay (Cmd+/)
- [ ] Multi-turn conversation branching
- [ ] Voice input
- [ ] Inline code suggestions
- [ ] Team/workspace-wide Claude Code connection
- [ ] Custom system prompts
- [ ] Integration with AgentMux workspace context

---

## Appendix A: Claude Code API Endpoints

**Authentication:**
- `GET https://claude.ai/code/auth` - OAuth authorization page
- `POST https://api.anthropic.com/v1/oauth/token` - Token exchange

**Messages:**
- `POST https://api.anthropic.com/v1/messages` - Send message, get streaming response

**Headers:**
```
Authorization: Bearer sk-ant-...
Content-Type: application/json
anthropic-version: 2024-01-01
```

**Request body:**
```json
{
  "model": "claude-opus-4",
  "messages": [
    { "role": "user", "content": "Hello" }
  ],
  "stream": true,
  "max_tokens": 4096
}
```

---

## Appendix B: References

- [Claude Code Documentation](https://docs.anthropic.com/claude-code)
- [Anthropic API Reference](https://docs.anthropic.com/api)
- [OAuth 2.0 Specification](https://oauth.net/2/)
- [Tauri Deep Link Guide](https://tauri.app/v1/guides/features/deep-link/)

---

**END OF SPECIFICATION**
