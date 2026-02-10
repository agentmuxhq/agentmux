<p align="center">
  <img src="./landing/logo.svg" alt="AgentMux Logo" width="120">
</p>

<h1 align="center">AgentMux</h1>

<p align="center">
  <b>AI-Native Agent Orchestrator</b> &mdash; Terminal multiplexer with reactive agent-to-agent messaging
</p>

<p align="center">
  <a href="https://opensource.org/licenses/Apache-2.0"><img src="https://img.shields.io/badge/License-Apache%202.0-blue.svg" alt="License"></a>
</p>

---

## What is AgentMux?

AgentMux is a Tauri v2 terminal multiplexer built for AI agent workflows:

- **Multi-pane terminal layouts** with per-pane agent identification
- **Reactive agent messaging** &mdash; inject messages directly into running Claude Code instances
- **Cross-host communication** via AgentMux cloud relay
- **Built-in AI chat** integration with OpenAI, Anthropic, and Google
- File previews, code editors, and embeddable widgets

Originally forked from [Wave Terminal](https://github.com/wavetermdev/waveterm).

## Quick Start

```bash
# Install dependencies
npm install --legacy-peer-deps

# Development (hot reload)
task dev

# Production build
task build
```

## Architecture

```
agentmux.exe (Tauri v2 - Rust + webview)
    └── agentmuxsrv (Go backend sidecar)
        └── wsh (shell integration CLI)
```

| Component | Size | Purpose |
|-----------|------|---------|
| `agentmux.exe` | ~14MB | Tauri frontend (Rust + native webview) |
| `agentmuxsrv` | ~33MB | Go backend (terminals, DB, AI, SSH) |
| `wsh` | ~11MB | Shell integration + remote RPC |
| **Total** | ~58MB | Compare: Electron version was ~135MB |

## Development

| Command | When to Use |
|---------|-------------|
| `task dev` | Normal development (hot reload) |
| `task build:backend` | After Go backend changes |
| `task build` | Production build with installer |
| `./bump-version.sh patch` | Version bump before release |

See [BUILD.md](BUILD.md) for full build instructions.

## License

Apache-2.0
