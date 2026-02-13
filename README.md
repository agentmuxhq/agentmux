<p align="center">
  <img src="./assets/wave-logo_icon-solid.svg" alt="AgentMux Logo" width="120">
</p>

# AgentMux

**AI-Native Terminal Multiplexer** - Fork of Wave Terminal

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Quick Start

```bash
# Install dependencies
npm install

# Development mode (hot reload)
task dev

# Production build
task package
```

## Build Commands

AgentMux uses [Task](https://taskfile.dev/) for build orchestration.

| Command | Description |
|---------|-------------|
| `task dev` | Start development mode with hot reload |
| `task package` | Build production installer (NSIS) |
| `task package:portable` | Build installer + portable ZIP |
| `task build:backend` | Build Go binaries only |
| `task build:frontend` | Build frontend only |
| `task test` | Run all tests |
| `task clean` | Clean build artifacts |

### npm Users

If you prefer npm:

```bash
npm run dev           # → task dev
npm run package       # → task package
npm run build:backend # → task build:backend
npm test              # → vitest (native)
```

### Build Outputs

- **Installer:** `src-tauri/target/release/bundle/nsis/AgentMux_*.exe`
- **Portable:** `dist/agentmux-*-portable.zip`
- **Standalone:** `src-tauri/target/release/agentmux.exe`

## Version Verification

The build script automatically verifies:
- `package.json` version matches built binaries
- All 8 wsh platform variants exist
- `wsh version` reports correct version
- Frontend and main process builds exist

## Architecture

```
AgentMux.exe (Tauri v2)
    └── agentmuxsrv.x64.exe (Go backend - sidecar, spawned automatically)
        └── wsh (shell integration - deployed to remotes)
```

**Stack:**
- **Frontend:** React + TypeScript + Vite
- **Backend:** Go (agentmuxsrv)
- **Desktop:** Tauri v2 (Rust + WebView2)
- **Build:** Task + npm + cargo

## Development

```bash
# Hot reload mode (auto-rebuilds on file changes)
task dev

# After Go changes, rebuild backend
task build:backend
# Then restart task dev

# Run tests
npm test

# Run with coverage
npm run coverage
```

## Prerequisites

**Required:**
- Node.js 20+
- Go 1.23+
- Rust 1.70+ (for Tauri)
- [Task](https://taskfile.dev/) - `npm install -g @go-task/cli`

**Windows-specific:**
- [Zig 0.13+](https://ziglang.org/download/) - For CGO cross-compilation
- WebView2 (usually pre-installed on Windows 10/11)

**Optional:**
- VS Code with recommended extensions

## License

Apache-2.0 - Originally forked from [Wave Terminal](https://github.com/wavetermdev/waveterm)
