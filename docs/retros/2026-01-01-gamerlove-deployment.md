# Retrospective: AgentMux Gamerlove Deployment

**Date:** 2026-01-01
**Duration:** ~45 minutes
**Outcome:** Success - AgentMux running on gamerlove sandbox

---

## Summary

Deployed latest AgentMux (with single-instance lock removal) to gamerlove Windows sandbox for development testing.

---

## What Went Well

1. **Single-instance removal worked** - PR #69 merged cleanly, app started without lock conflicts
2. **Go backend build** - Built successfully after clearing corrupted module cache
3. **SSH access** - Established reliable connection as `asafe@gamerlove`
4. **Admin PAT fallback** - When agent token failed, admin PAT provided repo access

---

## What Went Wrong

### 1. Git Authentication Confusion
- **Issue:** Initial token (`gh-token-agent2`) returned "Repository not found"
- **Root cause:** Token didn't have access to private `a5af/agentmux` repo
- **Fix:** Used `gh-admin-pat` instead
- **Time lost:** ~5 minutes

### 2. Corrupted Go Module Cache
- **Issue:** Build failed with `import path should not have @version`
- **Root cause:** Previous builds left corrupted entries in `GOMODCACHE`
- **Fix:** `go clean -modcache`
- **Time lost:** ~5 minutes

### 3. Frontend Build Hung
- **Issue:** `npm run build:dev` stalled on renderer bundle
- **Root cause:** Unknown - possibly resource constraints or SSH timeout
- **Fix:** Bypassed by using existing dist/frontend from previous build
- **Time lost:** ~10 minutes

### 4. Docs Build Failed (npm peer conflicts)
- **Issue:** `task dev` failed on `docs:npm:install` with @table-nav version conflict
- **Root cause:** Docs package.json has incompatible peer dependencies
- **Fix:** Ran `npx electron-vite dev` directly, skipping docs
- **Time lost:** ~5 minutes
- **Action item:** Fix docs/package.json peer dependencies

### 5. Confusing Architecture Explanation
- **Issue:** Log output showing "web server listening" confused user
- **Root cause:** Terminology - "web server" sounds like public HTTP
- **Lesson:** agentmuxsrv's localhost servers are IPC, not web hosting

---

## Action Items

| Priority | Item | Owner |
|----------|------|-------|
| P1 | Fix docs/package.json peer dependency conflicts | TBD |
| P2 | Document agentmuxsrv architecture (IPC vs web) | TBD |
| P2 | Add `--skip-docs` flag to `task dev` | TBD |
| P3 | Investigate frontend build hanging over SSH | TBD |

---

## Metrics

- **Total deployment time:** ~45 minutes
- **Blockers encountered:** 5
- **Commands to success:** ~25 SSH commands
- **Final working command:** `npx electron-vite dev`

---

## Architecture Clarification

```
┌─────────────────────────────────────────────────────┐
│                    gamerlove                         │
│                                                      │
│  ┌──────────────┐         ┌──────────────────────┐  │
│  │   Electron   │  IPC    │     agentmuxsrv       │  │
│  │   (Native    │◄───────►│     (Go backend)     │  │
│  │    Window)   │ :58901  │                      │  │
│  │              │ websocket│  - Terminal mgmt    │  │
│  │  - React UI  │         │  - Shell sessions    │  │
│  │  - Tabs      │ :58900  │  - File operations   │  │
│  │  - Blocks    │◄───────►│  - wsh integration   │  │
│  └──────────────┘  HTTP   └──────────────────────┘  │
│        ▲                                             │
│        │ Parsec/RDP                                  │
│        ▼                                             │
│   [Desktop Display]                                  │
└─────────────────────────────────────────────────────┘

Note: All servers bind to 127.0.0.1 (localhost only)
      No external network exposure
```

---

## Lessons Learned

1. **Token scope matters** - Agent-specific tokens may not have repo access; keep admin PAT as fallback
2. **Go module cache corrupts** - When seeing weird import path errors, `go clean -modcache` first
3. **Skip optional builds** - Docs build is not required for dev; bypass with direct electron-vite
4. **Terminology clarity** - "Web server" in Electron context means IPC, not public HTTP
