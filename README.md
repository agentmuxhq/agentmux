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
- **Version:** 0.12.15
- **License:** Apache-2.0

## Architecture

- **Electron 38.1.2** - Desktop application framework
- **TypeScript + React** - Frontend UI
- **Go** - Backend server (wavesrv) and shell integration (wsh)
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

# Build Go binaries (wavesrv, wsh)
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
