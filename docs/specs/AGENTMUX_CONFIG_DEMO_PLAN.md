# AgentMux Configuration Demonstration Plan

This document provides a step-by-step demonstration plan for testing the WaveMux 0.16.7 agentmux auto-configuration and runtime reconfiguration features.

## Prerequisites

- WaveMux 0.16.7 installed
- Access to AgentMux server (https://agentmux.asaf.cc)
- Valid token: `<YOUR_AGENTMUX_TOKEN>`

---

## Part 1: Auto-Configuration from File (Startup)

### Test 1.1: Fresh Start with Config File

1. **Close WaveMux completely**

2. **Create config file** at `~/.waveterm/agentmux.json`:
   ```json
   {
     "url": "https://agentmux.asaf.cc",
     "token": "<YOUR_AGENTMUX_TOKEN>"
   }
   ```

   On Windows: `%USERPROFILE%\.waveterm\agentmux.json`
   On Linux/Mac: `~/.waveterm/agentmux.json`

3. **Start WaveMux**

4. **Verify poller started** - Check logs for:
   ```
   [reactive/poller] loaded config from file: URL=https://agentmux.asaf.cc
   [reactive/poller] started polling https://agentmux.asaf.cc every 5s
   ```

5. **Expected result**: Poller auto-starts without any manual configuration

### Test 1.2: No Config File

1. **Delete or rename** `~/.waveterm/agentmux.json`
2. **Start WaveMux**
3. **Verify poller NOT started** - Check logs for:
   ```
   [reactive/poller] cross-host polling disabled (no AGENTMUX_URL)
   ```

---

## Part 2: Runtime Configuration via wsh Command

### Test 2.1: Configure at Runtime

1. **Start WaveMux** (without config file or env vars)

2. **Open terminal** in WaveMux

3. **Run configuration command**:
   ```bash
   wsh agentmux config https://agentmux.asaf.cc "<YOUR_AGENTMUX_TOKEN>"
   ```

4. **Verify output**:
   ```
   AgentMux configured: https://agentmux.asaf.cc
   ```

5. **Verify poller started** - Check logs for:
   ```
   [reactive/poller] reconfigured: URL=https://agentmux.asaf.cc
   [reactive/poller] started polling https://agentmux.asaf.cc every 5s
   ```

6. **Verify config file created** - Check `~/.waveterm/agentmux.json` exists with correct content

### Test 2.2: Change Configuration at Runtime

1. **With poller already running**, change to a different URL:
   ```bash
   wsh agentmux config https://different-server.example.com "newtoken123"
   ```

2. **Verify poller restarted** with new URL - Check logs for:
   ```
   [reactive/poller] stopped
   [reactive/poller] reconfigured: URL=https://different-server.example.com
   [reactive/poller] started polling...
   ```

3. **Verify config file updated**

### Test 2.3: Disable Polling

1. **Run clear command**:
   ```bash
   wsh agentmux config clear
   ```

2. **Verify output**:
   ```
   AgentMux cross-host polling disabled
   ```

3. **Verify poller stopped** - Check logs for:
   ```
   [reactive/poller] stopped
   [reactive/poller] cross-host polling disabled (URL cleared)
   ```

---

## Part 3: Cross-Agent Injection Test

### Test 3.1: Receive Injection

1. **Ensure poller is configured and running** on target agent

2. **From another agent**, send injection:
   ```bash
   curl -X POST https://agentmux.asaf.cc/reactive/inject \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer <YOUR_AGENTMUX_TOKEN>" \
     -H "X-Agent-ID: source-agent" \
     -d '{"target_agent": "TARGET_AGENT_ID", "message": "Hello from remote agent!", "priority": "normal"}'
   ```

3. **Verify injection received** on target agent within 5 seconds (poll interval)

### Test 3.2: Configuration Persistence

1. **Configure poller** via `wsh agentmux config`
2. **Restart WaveMux**
3. **Verify poller auto-starts** with saved configuration (no manual config needed)

---

## Part 4: Security Validation

### Test 4.1: URL Validation

1. **Attempt invalid URL**:
   ```bash
   wsh agentmux config "file:///etc/passwd" "token"
   ```
   Expected: Error - URL scheme must be https

2. **Attempt HTTP to non-localhost**:
   ```bash
   wsh agentmux config "http://evil.com" "token"
   ```
   Expected: Error - http:// only allowed for localhost

3. **HTTP localhost should work** (dev mode):
   ```bash
   wsh agentmux config "http://localhost:8080" "token"
   ```
   Expected: Success (for development)

---

## Verification Checklist

- [ ] Config file auto-loads on startup
- [ ] No config file = poller disabled
- [ ] `wsh agentmux config` enables poller at runtime
- [ ] `wsh agentmux config clear` disables poller
- [ ] Config changes persist to file
- [ ] Restart loads saved config
- [ ] Cross-agent injection works when poller is running
- [ ] Invalid URLs are rejected (SSRF protection)
- [ ] HTTP only allowed for localhost

---

## Quick Commands Reference

```bash
# Configure (saves to file automatically)
wsh agentmux config https://agentmux.asaf.cc "TOKEN"

# Disable
wsh agentmux config clear

# Check status instructions
wsh agentmux status

# Manual status check (if you know the port)
curl http://localhost:$WAVETERM_DEV_PORT/wave/reactive/poller/status
```

---

## Config File Location

| Platform | Path |
|----------|------|
| Windows | `%USERPROFILE%\.waveterm\agentmux.json` |
| macOS | `~/.waveterm/agentmux.json` |
| Linux | `~/.waveterm/agentmux.json` |

## Config File Format

```json
{
  "url": "https://agentmux.asaf.cc",
  "token": "your-token-here"
}
```
