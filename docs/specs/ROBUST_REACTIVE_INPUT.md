# Robust Reactive Input Injection

**Date:** 2026-01-16
**Author:** AgentA
**Status:** Proposed
**Priority:** High

---

## Problem

Reactive injections send text to the terminal PTY, but the "Enter" key isn't reliably processed. Text appears in the terminal but Claude doesn't receive it as input.

**Current behavior:**
1. Message sent to PTY via `inputSender(blockID, []byte(finalMsg))`
2. After 100ms, Enter key sent: `inputSender(blockID, []byte("\r"))`
3. Text appears in terminal visually
4. BUT Claude often doesn't process it as user input

**Root cause hypotheses:**
1. 100ms delay insufficient - Claude not ready to receive Enter
2. `\r` wrong line ending - might need `\n` or `\r\n`
3. Terminal buffer not flushed before Enter
4. Claude Code input handler expects different signaling
5. PTY vs user input path differs in how Claude processes it

---

## Proposed Solutions

### Option A: Increase Delay + Retry (Quick Fix)

```go
// Send Enter key with retry
go func() {
    delays := []time.Duration{200*time.Millisecond, 500*time.Millisecond, 1*time.Second}
    for _, delay := range delays {
        time.Sleep(delay)
        if err := h.inputSender(blockID, []byte("\r\n")); err != nil {
            log.Printf("[reactive] Enter send failed: %v", err)
            continue
        }
        // TODO: Add verification that input was processed
        break
    }
}()
```

**Pros:** Simple change
**Cons:** Still unreliable, wastes time on delays

### Option B: Use Different Line Endings

Try multiple line ending formats:
```go
// Try different Enter sequences
lineEndings := [][]byte{
    []byte("\r"),      // CR
    []byte("\n"),      // LF
    []byte("\r\n"),    // CRLF
    []byte("\x04"),    // Ctrl+D (EOF)
}
```

### Option C: Send via WebSocket/RPC Instead of PTY

Instead of injecting into PTY, send through the existing RPC/WebSocket system that Claude Code uses for normal user input.

```go
// New approach: use RPC message instead of PTY
func (h *Handler) InjectMessageViaRPC(req InjectionRequest) InjectionResponse {
    // Send as a proper "user message" through the RPC system
    // This goes through the same path as normal user input
    rpcMsg := wshrpc.RpcMessage{
        Command: "terminal:input",
        Data: map[string]interface{}{
            "blockId": blockID,
            "input":   finalMsg,
            "submit":  true,  // Flag to auto-submit
        },
    }
    // Send via WebSocket
}
```

**Pros:** Uses same path as real user input
**Cons:** Requires more investigation of RPC system

### Option D: Verification + Retry Loop

Add verification that the message was actually processed:

```go
func (h *Handler) InjectMessageWithVerification(req InjectionRequest) InjectionResponse {
    // Send message
    h.inputSender(blockID, []byte(finalMsg))

    // Send Enter
    h.inputSender(blockID, []byte("\r\n"))

    // Wait and verify
    for i := 0; i < 5; i++ {
        time.Sleep(500 * time.Millisecond)
        if h.verifyMessageProcessed(blockID, finalMsg) {
            return successResponse()
        }
        // Retry Enter
        h.inputSender(blockID, []byte("\r\n"))
    }

    return errorResponse("message not processed after 5 retries")
}
```

### Option E: Terminal Focus + Explicit Submit

Ensure terminal has focus before sending Enter:

```go
// 1. Focus the terminal block
h.focusBlock(blockID)

// 2. Small delay for focus to take effect
time.Sleep(50 * time.Millisecond)

// 3. Send message
h.inputSender(blockID, []byte(finalMsg))

// 4. Explicit submit signal
time.Sleep(100 * time.Millisecond)
h.inputSender(blockID, []byte("\r\n"))
```

---

## Recommended Approach

**Start with Option A + B combined:**

```go
// InjectMessage - updated implementation
func (h *Handler) InjectMessage(req InjectionRequest) InjectionResponse {
    // ... validation ...

    // Send message content
    err := h.inputSender(blockID, []byte(finalMsg))
    if err != nil {
        return h.errorResponse(req, fmt.Sprintf("failed to send input: %v", err))
    }

    // Log successful message delivery
    h.logAudit(req, blockID, len(finalMsg), true, "")

    // Send Enter with retry in background
    go func() {
        // Wait for message to be fully written to PTY
        time.Sleep(250 * time.Millisecond)

        // Try CRLF first (most compatible)
        if err := h.inputSender(blockID, []byte("\r\n")); err != nil {
            log.Printf("[reactive] Enter send failed: %v", err)
        }

        // If still not processed after 500ms, try again
        time.Sleep(500 * time.Millisecond)
        if err := h.inputSender(blockID, []byte("\r\n")); err != nil {
            log.Printf("[reactive] Enter retry failed: %v", err)
        }
    }()

    return successResponse()
}
```

---

## Testing

1. Send injection to agent
2. Verify message appears in terminal
3. Verify Claude processes it as input (responds)
4. Test with various message lengths
5. Test rapid-fire injections
6. Test cross-host injections

---

## Implementation Checklist

- [ ] Update `InjectMessage` in `pkg/reactive/handler.go`
- [ ] Change delay from 100ms to 250ms
- [ ] Change `\r` to `\r\n`
- [ ] Add retry after 500ms
- [ ] Add logging for debugging
- [ ] Test locally
- [ ] Test cross-host
- [ ] Bump version

---

## Future Work

If Option A+B still unreliable, investigate Option C (RPC-based input) which would bypass PTY entirely and use the same input path as real user messages.
