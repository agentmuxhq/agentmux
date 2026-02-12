# AgentMux Version History

This document tracks the version history of the a5af/agentmux fork (formerly a5af/waveterm).

## Latest Version: 0.26.0

**Base:** Upstream waveterm v0.12.0 + extensive custom features

---

## Version History (Latest First)

### v0.26.0 (2026-02-12)
- **Agent:** AgentClaude
- **Changes:**
  - Feat: Display AgentMux version in tabbar (centered, clickable to copy)
  - Feat: Enable window dragging from entire tabbar area
  - Feat: Add right-click context menu to toggle widget visibility
  - Fix: Add macOS-specific version bump script (bump-version-osx.sh)

### v0.16.7 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Feat: Auto-load agentmux config from file on startup
  - Add LoadAgentMuxConfigFile() to load ~/.waveterm/agentmux.json
  - Add SaveAgentMuxConfigFile() to persist runtime config changes
  - ReconfigureGlobalPoller() now saves config to file automatically
  - No pre-configuration needed - just place agentmux.json and restart
  - Priority: config file < env vars (env vars override file config)

### v0.16.6 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Feat: Runtime agentmux config via wsh agentmux command
  - Add ReconfigureGlobalPoller() for runtime poller updates
  - Add HTTP endpoints: /wave/reactive/poller/config, /status
  - Add OSC 16162 "X" command for agentmux config
  - New wsh commands: `wsh agentmux config`, `wsh agentmux status`
  - Allows configuring AgentMux without restarting AgentMux

### v0.16.5 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Fix: Revert to synchronous Enter key for reactive injection
  - Add rate limiter (10 req/sec) for DoS protection
  - Docs: Add REACTIVE_INJECTION_REGRESSION_REPORT.md

### v0.16.4 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Fix: Enter key retry with 3 attempts (still broken)
  - Added documentation for the issue

### v0.16.3 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Fix: Enter key timing for reactive injection (300ms delay, CRLF)
  - Added retry after 700ms (still broken)

### v0.16.2 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Feat: Made Enter key async to prevent DoS (breaking change)
  - This change broke message/Enter coordination

### v0.16.1 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Feat: Cross-host reactive messaging poller (#144)
  - AgentMux polls AgentMux for pending injections from remote agents
  - New endpoint: /wave/reactive/poller/stats for monitoring
  - Configurable via AGENTMUX_URL, AGENTMUX_TOKEN env vars
  - Enables agent-to-agent messaging across different machines

### v0.16.0 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Feat: Reactive agent-to-agent messaging (#140, #141)
  - Inject messages directly into running Claude Code instances
  - New HTTP API: /wave/reactive/inject, /agents, /register, /unregister, /audit
  - Frontend auto-registers agents via OSC 16162 WAVEMUX_AGENT_ID
  - Message sanitization and audit logging
  - AgentMux inject_terminal MCP tool (agentmux#69)

### v0.15.15 (2026-01-16)
- **Agent:** AgentX
- **Changes:**
  - Feat: add WAVEMUX_AGENT_TEXT_COLOR support for pane header text (#137)
  - Customizable text color for agent pane headers
  - Smart defaults: white text on dark backgrounds, black on light

### v0.15.14 (2026-01-16)
- **Agent:** AgentA
- **Changes:**
  - Refactor: remove AGENTMUX_AGENT_ID coupling (#135)
  - AgentMux now only uses WAVEMUX_AGENT_ID for agent identity
  - Shell integration scripts cleaned of AGENTMUX fallbacks

### v0.15.13 (2026-01-15)
- **Agent:** AgentA
- **Changes:**
  - Fix: prevent duplicate title display in pane header and titlebar (#134)
  - Fix: decouple system hostname from agent detection (#132)

### v0.15.12 (2026-01-15)
- **Agent:** AgentA
- **Changes:**
  - Docs: update VERSION_HISTORY.md to reflect current state (#130)

### v0.15.9 - v0.15.11 (2026-01-15)
- **Agent:** AgentA
- **Changes:**
  - Fix: disable hostname-based agent detection for local terminals (#127)
  - Local terminals no longer auto-detect agent from hostname patterns
  - Explicit `agent-workspaces` directory pattern still works
  - Env vars (WAVEMUX_AGENT_ID) take highest priority

### v0.15.5 - v0.15.8 (2026-01-14)
- **Agent:** AgentX
- **Changes:**
  - Fix: Claude activity display - no duplicate, bold in header (#126)
  - Fix: per-pane agent identification + build system fixes (#125)
  - Fix: re-enable hardware acceleration by default (#123)

### v0.15.4 (2026-01-13)
- **Agent:** AgentX
- **Changes:**
  - Feat: add AgentY to default agent colors (#122)
  - Feat: Display Claude activity summaries in pane title bar (#121)
  - Feat: per-pane agent colors via shell environment variables (#120)
  - Fix: improve agent detection path matching with trailing slash (#119)

### v0.15.0 - v0.15.3 (2026-01-12)
- **Agent:** AgentX
- **Changes:**
  - Feat: Add agent colors to terminal pane headers (#103)
  - Feat: environment variable-based agent detection (#102)
  - Disable Dependabot - causing too many blockers (#118)
  - Sync missing aiprompts files from upstream waveterm

### v0.14.0 (2026-01-09)
- **Agent:** Agent2
- **Changes:**
  - Removed Storybook (unused dev tool, ~36MB savings)
  - Removed Storybook references from Dependabot config
  - Fixed gamerlove startup failures (reverted to simple 1-terminal layout)
  - Disabled hardware acceleration for Windows Sandbox/RDP compatibility
  - Added console window with verbose startup logging
  - Multiple dependency updates (xterm, monaco, react-hook-form, etc.)

### v0.13.3 - v0.13.6 (2026-01-08)
- **Changes:**
  - Various hardware acceleration and startup fixes
  - Window size calculation debugging
  - Layout fixes for gamerlove

### v0.12.14-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Agent:** agentx
- **Changes:**
  - **P0 FIX:** Cross-platform wsh binary exclusions (breaks macOS/Linux builds)
  - **P1 FIX:** Updater IPC handler crash when auto-update disabled
  - Added RELEASE_CHECKLIST.md with comprehensive workflow guide
  - Enhanced bump-version.sh to prevent releasing old code
  - Documented correct release workflow to prevent v0.12.13 issue recurrence

### v0.12.13-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Agent:** agentx
- **Changes:**
  - Fix title bar instance number parsing bug (was showing "undefined")
  - Add comprehensive app name and instance tests
  - **NOTE:** This version was released BEFORE instance parsing fix was committed
  - **ISSUE:** Users downloaded old code under new version number
  - **RESOLUTION:** v0.12.14 includes all fixes with corrected workflow

### v0.12.12-fork (2025-10-20)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Changes:**
  - Package verification and version consistency fixes

### v0.12.11-fork (2025-10-20)
- **Changes:**
  - Version management improvements and documentation

### v0.12.10-fork (2025-10-19)
- **Location:** Main worktree (`D:/Code/waveterm`)
- **Branch:** `feature/high-contrast-terminal-borders`
- **Changes:**
  - Fix waveConfigDirName undefined error
  - Add smoke tests for configuration

### v0.12.9-fork (2025-10-19)
- **Changes:**
  - Fix waveDirName undefined error
  - Add more configuration tests

### v0.12.8-fork (2025-10-19)
- **Changes:**
  - Implement portable multi-instance mode with persistent settings

### v0.12.7-fork (2025-10-19)
- **Changes:**
  - UI improvements: hard corners, better borders
  - Fix settings persistence issues
  - Optimize build size: remove heavy artifacts

### v0.12.6-fork (2025-10-19)
- **Changes:**
  - Add comprehensive crash reporting system

### v0.12.3-fork (2025-10-19)
- **Location:** This worktree (`D:/Code/agent-workspaces/agentx/waveterm`)
- **Agent:** agentx
- **Branch:** `agentx/merge-upstream-v0.12.0`
- **Changes:**
  - Add high-contrast white borders to unselected terminal blocks
  - Fix electron-builder packaging bug (upgrade to v26.1.0)
  - Document critical electron-builder files configuration bug
  - Add build investigation spec and artifact verification
  - Added multi-instance development support
  - Added comprehensive documentation (BUILD.md, CLAUDE.md)
  - **Added version management scripts (bump-version.sh/ps1) and this VERSION_HISTORY.md**

### v0.12.2-fork
- Multi-instance support improvements
- Multi-instance dialog

### v0.12.1-fork
- Inherit main install profile
- Initial multi-instance support with shared config

### v0.12.0-fork
- Initial merge from upstream v0.12.0

---

## Development Setup

**Primary Development Machine:** area54 (192.168.1.26)
**Repository:** `C:\Systems\agentmux`

### Agent Workspaces

Agents work on feature branches from `main`:
- `agenta/feature-name` - AgentA (area54)
- `agentx/feature-name` - AgentX (claudius)
- `agentg/feature-name` - AgentG (gamerlove)

---

## Upstream Version Tracking

- **Upstream repository:** https://github.com/wavetermdev/waveterm
- **Base Upstream Version:** v0.12.0
- **Fork repository:** https://github.com/a5af/agentmux
- **Latest Fork:** v0.15.15
- **Commits Ahead of Upstream:** 100+ commits with custom features

---

## Key Fork Features

1. **Per-pane agent identification** - Terminal panes show agent identity (AgentA, AgentX, etc.)
2. **Agent color borders** - Colored borders indicate which agent owns a pane
3. **Claude activity display** - Shows Claude Code activity summaries in title bar
4. **Environment-based agent detection** - WAVEMUX_AGENT_ID env var
5. **OSC 16162 shell integration** - Shell can send agent identity via escape sequences
6. **Multi-instance support** - Multiple AgentMux instances can run simultaneously
7. **Portable mode** - Persistent settings across instances
8. **High-contrast borders** - Visual improvements for terminal blocks
9. **Version management** - Automated version bump scripts

---

## Version Bump Instructions

```bash
# Bump patch version (0.15.8 -> 0.15.9)
./bump-version.sh patch --message "Fix description"

# Bump minor version (0.15.8 -> 0.16.0)
./bump-version.sh minor --message "New feature"
```

The bump scripts automatically:
- ✅ Update `package.json` and `package-lock.json`
- ✅ Create git commit with version message
- ✅ Create git tag (e.g., `v0.15.9-fork`)

---

## Notes for Agents

- Always check this file first to understand current version state
- Create feature branches from `main`: `git checkout -b agentX/feature-name`
- Open PRs against `main` branch (it's protected, requires PR)
- Run `task build:backend` after Go changes
- Run `task dev` for development with hot reload
- Run `task package` only for final release builds
