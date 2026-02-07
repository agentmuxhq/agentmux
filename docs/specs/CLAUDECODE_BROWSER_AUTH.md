# Claude Code Browser Authentication Spec

## Problem

When Claude Code starts for the first time (or when the session/subscription expires), it outputs a URL that the user must open in a browser to authenticate. In a standard terminal, the user can click or copy the URL. In WaveMux's Claude Code pane, the stream-json output may not surface this URL in a clickable way, blocking the user from completing authentication.

## Background

### Claude Code Authentication Flow

1. User runs `claude` CLI
2. If no valid session exists, Claude Code outputs:
   ```
   To sign in, open this URL in your browser:
   https://console.anthropic.com/oauth/authorize?...
   ```
3. User opens the URL in a browser
4. Browser completes OAuth flow (login / subscription confirmation)
5. Claude Code receives the auth token and begins the session
6. Token is cached in `~/.claude/` for future sessions

### Current WaveMux Behavior

- The `claudecode` pane uses `--output-format stream-json`
- Auth prompts may appear as `system` events with `subtype: "auth"` or as raw text before the stream-json protocol starts
- The raw terminal view (`[term]` toggle) shows all output, but the parsed chat view may miss pre-protocol text

## Design

### Detection Strategy

There are two places auth URLs can appear:

1. **Pre-protocol output**: Before stream-json starts, Claude Code may emit plain text (not JSON). The parser currently ignores non-JSON lines. We need to capture these and check for auth URLs.

2. **System events**: The stream-json protocol may emit `{"type":"system","subtype":"auth","message":"...","url":"..."}` events. Our parser already handles system events.

### Implementation Plan

#### Phase 1: URL Detection in Parser

Add an `onRawLine` callback to `ParserCallbacks`:

```typescript
interface ParserCallbacks {
    // ... existing callbacks ...
    onRawLine?: (line: string) => void;  // Non-JSON lines (pre-protocol text)
}
```

In `ClaudeCodeStreamParser.parseLine()`, instead of silently ignoring JSON parse failures, forward the raw line:

```typescript
private parseLine(line: string): void {
    let parsed: any;
    try {
        parsed = JSON.parse(line);
    } catch {
        this.callbacks.onRawLine?.(line);  // Forward non-JSON lines
        return;
    }
    // ... rest of parsing
}
```

#### Phase 2: URL Extraction & Auth State

In the ViewModel, add auth state tracking:

```typescript
// New atoms
authUrlAtom: PrimitiveAtom<string>;  // "" when no auth needed
authStateAtom: Atom<"none" | "pending" | "complete">;
```

The `onRawLine` callback and `onSystemEvent` callback both check for auth URLs:

```typescript
onRawLine: (line: string) => {
    // Match URLs from auth prompts
    const urlMatch = line.match(/https:\/\/console\.anthropic\.com\/[^\s]+/);
    if (urlMatch) {
        globalStore.set(this.authUrlAtom, urlMatch[0]);
    }
},

onSystemEvent: (event: SystemEvent) => {
    if (event.subtype === "auth" && event.url) {
        globalStore.set(this.authUrlAtom, event.url);
    }
    // Clear auth state when session starts
    if (event.session_id) {
        globalStore.set(this.authUrlAtom, "");
    }
    // ... existing handling
}
```

#### Phase 3: Auth Banner UI

Add an `AuthBanner` component that renders when `authUrlAtom` is non-empty:

```
┌─────────────────────────────────────────────────────────┐
│ ⚡ Claude Code requires authentication                  │
│                                                         │
│ [Open Browser]  [Copy URL]                             │
│                                                         │
│ https://console.anthropic.com/oauth/authorize?...      │
└─────────────────────────────────────────────────────────┘
```

**Open Browser** button: Uses Electron's `shell.openExternal(url)` via the WaveMux RPC bridge. The existing `getApi().openExternalLink(url)` method handles this.

**Copy URL** button: Copies the URL to clipboard via `navigator.clipboard.writeText(url)`.

The banner auto-dismisses when:
- A `system` event with `session_id` arrives (auth succeeded)
- A `message_start` event arrives (session is active)

#### Phase 4: Subscription Flow

For users without a Claude subscription:

1. The auth URL redirects to Anthropic's sign-in page
2. After sign-in, if no active subscription exists, Claude Code will either:
   - Show an error in the stream output (captured by `onError` callback)
   - Exit with a non-zero exit code (captured by `shellProcExitCodeAtom`)
3. The error message should indicate subscription is required
4. The StatusBar already shows exit codes; an `ErrorBanner` will display the error message

### Edge Cases

| Scenario | Handling |
|----------|----------|
| Auth URL in middle of session | Show banner, don't clear conversation |
| Multiple auth URLs | Use most recent URL |
| User dismisses banner | Add dismiss button, store in session state |
| Auth timeout | Claude Code handles this internally; exits with error |
| Token refresh | Usually silent; if interactive, same auth flow |
| Offline | Show connection error via ErrorBanner |

### Security Considerations

- Only open URLs matching `https://console.anthropic.com/*` or `https://auth.anthropic.com/*`
- Never log or display full OAuth tokens
- Auth state is per-block, not persisted across sessions

## File Changes

| File | Change |
|------|--------|
| `claudecode-parser.ts` | Add `onRawLine` callback |
| `claudecode-types.ts` | Add `url` field to `SystemEvent` |
| `claudecode-model.ts` | Add `authUrlAtom`, handle auth detection |
| `claudecode-view.tsx` | Add `AuthBanner` component |
| `claudecode.scss` | Add `.cc-auth-banner` styles |

## Testing

1. Kill any cached auth: `rm -rf ~/.claude/`
2. Open Claude Code pane in WaveMux
3. Verify auth banner appears with the URL
4. Click "Open Browser" — browser should open to Anthropic auth
5. Complete auth flow in browser
6. Verify banner auto-dismisses and Claude Code session starts
7. Close and reopen pane — should not prompt for auth again (cached token)

## Open Questions

1. Does `--output-format stream-json` change the auth flow output format, or does auth happen before the protocol starts?
2. Should we support `claude --api-key` as an alternative to browser auth?
3. Is there a way to check auth status before starting the process (e.g., checking `~/.claude/` for valid tokens)?
