<p align="center">
  <img src="./assets/wave-logo_icon-solid.svg" alt="WaveMux Logo" width="120">
  <br/>
  <br/>
</p>

# 🌊 WaveMux

**Terminal Multiplexer** - Independent fork of Wave Terminal

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## About

WaveMux is an independent terminal multiplexer project, originally forked from [Wave Terminal](https://github.com/wavetermdev/waveterm). This project preserves all enhancements developed in the v0.12.15 fork, including:

- Dynamic version display fixes
- Enhanced wsh binary error handling
- Comprehensive e2e test suite (14 tests)
- Improved shell integration

## Repository

- **GitHub:** https://github.com/a5af/wavemux
- **Version:** 0.13.0
- **License:** Apache-2.0

## Agent Development Workflow (Gamerlove Sandbox)

WaveMux runs on **gamerlove** (sandbox host) while code is edited on **claudius** (development host).

### Quick Sync Commands

```powershell
# From claudius - sync source files to gamerlove sandbox
robocopy C:\Code\agent-workspaces\agent2\wavemux X:\wavemux-sandbox /MIR /XD node_modules dist .task .git /NFL /NDL /NJH /NJS

# Build backend on gamerlove (after Go changes)
ssh gamerlove "powershell -Command Set-Location D:\wavemux-sandbox; task build:backend"

# Start dev server on gamerlove
ssh gamerlove "powershell -Command Set-Location D:\wavemux-sandbox; task dev"
```

### Architecture Overview

```
CLAUDIUS (Dev Host)                    GAMERLOVE (Sandbox)
─────────────────────                  ────────────────────
C:\Code\agent-workspaces\              D:\wavemux-sandbox\
  └── agent2\wavemux\     ──sync──>      ├── dist\bin\wavemuxsrv.x64.exe
      ├── pkg/ (Go)                      ├── node_modules\
      ├── frontend/ (TS)                 └── WaveMux running (Electron)
      └── .git (worktree)

X:\wavemux-sandbox\ = mapped drive to gamerlove D$
```

### What Triggers What

| Change Type | Sync | Build Backend | Restart Dev |
|-------------|------|---------------|-------------|
| TypeScript/React | ✅ | ❌ | ❌ (hot reload) |
| Go backend (pkg/) | ✅ | ✅ | ✅ |
| package.json | ✅ | ❌ | ✅ (npm install) |

> **Full documentation:** See `SPEC_GAMERLOVE_WAVEMUX_WORKFLOW.md` in agent2 workspace

## Architecture

- **Electron 38.1.2** - Desktop application framework
- **TypeScript + React** - Frontend UI
- **Go** - Backend server (wavemuxsrv) and shell integration (wsh)
- **Vite** - Build tool
- **Vitest** - Testing framework

## Development

### Prerequisites

- Node.js 18+ with npm
- Go 1.21+
- Task (taskfile.dev)

### Build Commands

```bash
# Install dependencies
npm install

# Development mode (hot reload)
task dev

# Build Go binaries (wavemuxsrv, wsh)
task build:backend

# Build frontend (production)
npm run build:prod

# Run tests
npm test

# E2E tests
npm test -- app.e2e.test.ts

# Package release
task package
```

### Version Management

```bash
# Bump version
./bump-version.sh patch --message "Description"

# Rebuild binaries after version bump
task build:backend

# Verify consistency
bash scripts/verify-version.sh
```

## Key Files

- `package.json` - Project metadata
- `CLAUDE.md` - Development guide for AI agents
- `VERSION_HISTORY.md` - Version changelog
- `Taskfile.yml` - Build tasks
- `emain/app.e2e.test.ts` - E2E test suite

## Testing

All 14 e2e tests pass successfully:
- Version display verification
- wsh binary deployment checks
- Shell integration tests
- Error handling validation

## License

WaveMux is licensed under the Apache-2.0 License, inherited from the original Wave Terminal project.

---

*This is an independent project. Original Wave Terminal: https://github.com/wavetermdev/waveterm*
