<p align="center">
  <img src="./assets/wave-logo_icon-solid.svg" alt="AgentMux Logo" width="120">
</p>

# AgentMux

**AI-Native Terminal Multiplexer** - Fork of Wave Terminal

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Quick Start

```powershell
# Install dependencies
npm install --legacy-peer-deps

# Build and package (one command)
.\scripts\build-release.ps1 -Clean
```

Output: `make\win-unpacked\AgentMux.exe`

## Build Commands

| Command | Purpose |
|---------|---------|
| `.\scripts\build-release.ps1 -Clean` | Full clean build with verification |
| `task build:backend` | Rebuild Go binaries only |
| `npm run build:prod` | Rebuild frontend only |
| `task dev` | Development mode (hot reload) |

## Build Script Options

```powershell
.\scripts\build-release.ps1 [-Clean] [-SkipBackend] [-SkipFrontend] [-SkipPackage]
```

- `-Clean` - Kill processes, remove stale artifacts
- `-SkipBackend` - Skip Go build (use existing binaries)
- `-SkipFrontend` - Skip TypeScript build
- `-SkipPackage` - Skip electron-builder packaging

## Version Verification

The build script automatically verifies:
- `package.json` version matches built binaries
- All 8 wsh platform variants exist
- `wsh version` reports correct version
- Frontend and main process builds exist

## Architecture

```
AgentMux.exe (Electron)
    └── agentmuxsrv.x64.exe (Go backend - spawned automatically)
        └── wsh (shell integration - deployed to remotes)
```

## Development

```powershell
# Hot reload mode
task dev

# After Go changes
task build:backend
# Then restart task dev

# Run tests
npm test
```

## Deploy to Desktop

```powershell
$Version = (Get-Content package.json | ConvertFrom-Json).version
xcopy /E /Y /I make\win-unpacked "C:\Users\asafe\Desktop\AgentMux-$Version\"
```

## Prerequisites

- Node.js 18+ with npm
- Go 1.21+
- Task (taskfile.dev)
- Windows with zig (for cross-compilation)

## License

Apache-2.0 - Originally forked from [Wave Terminal](https://github.com/wavetermdev/waveterm)
