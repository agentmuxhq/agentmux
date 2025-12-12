# @a5af/sandbox - WaveMux Development Tools

WaveMux development sandbox setup and management tools.

## Overview

This package provides automation scripts to configure a remote Windows machine as an isolated WaveMux development environment. This allows you to develop and test WaveMux without risking crashes on your main workstation.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    MAIN WORKSTATION                          │
│                                                              │
│  WaveMux (--instance=main)  ←──→  Parsec Client             │
│  Production agent interaction        Low-latency remote      │
└──────────────────────────────────────│───────────────────────┘
                                       │
                    Parsec P2P (4-8ms latency)
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────┐
│                    SANDBOX HOST                              │
│                                                              │
│  WaveMux (--instance=dev)   ←──→  Parsec Host               │
│  Development & testing              Always-connected VDA     │
│                                                              │
│  Tools: Node.js, Go, Zig, Task, Git, VS Code                │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### On Sandbox Host (run once)

```powershell
# Full setup - installs everything
setup-sandbox-host

# Skip Parsec if already installed
setup-sandbox-host -SkipParsec

# Skip dev tools if already installed
setup-sandbox-host -SkipDevTools

# Clone specific WaveMux branch
setup-sandbox-host -WaveMuxBranch agentx/feature
```

### Health Check

```powershell
sandbox-health              # Quick check
sandbox-health -Verbose     # Detailed output
sandbox-health -OutputFormat json  # For automation
```

## What Gets Installed

### Development Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | 18 LTS | Frontend build |
| Go | 1.21+ | Backend build |
| Zig | latest | Native compilation |
| Task | latest | Task runner |
| Git | latest | Version control |
| VS Code | latest | Editor |

### Remote Access

| Component | Purpose |
|-----------|---------|
| Parsec | Low-latency remote desktop (4-8ms) |
| Parsec VDA | Virtual display for headless operation |

### WaveMux

- Cloned to `D:\Code\sandbox\wavemux`
- Configured with `--instance=dev` for isolation
- Data stored in `~/.wavemux-dev/` (separate from production)

## Daily Workflow

```powershell
# 1. Connect to sandbox via Parsec from main workstation

# 2. Navigate to WaveMux
cd D:\Code\sandbox\wavemux

# 3. Start development server
task dev

# 4. Launch WaveMux with dev instance
wavemux --instance=dev

# 5. Make changes, test, repeat
# Frontend changes hot-reload automatically
# Backend changes require: task build:backend && restart task dev
```

## Scripts

### bin/setup-sandbox-host.ps1

Main entry point for sandbox setup. Wrapper that finds and executes the implementation script.

**Parameters:**
- `-SkipParsec` - Skip Parsec installation
- `-SkipDevTools` - Skip development tools installation
- `-Force` - Reinstall even if already present
- `-Verbose` - Enable detailed output
- `-WaveMuxBranch <branch>` - Clone specific branch (default: main)

### bin/sandbox-health.ps1

Health check wrapper for verifying sandbox configuration.

**Parameters:**
- `-Verbose` - Enable detailed output
- `-OutputFormat <text|json>` - Output format

## Configuration

### config/parsec-config.json

Parsec host configuration template for headless operation:

```json
{
  "host_virtual_monitor": 1,
  "host_virtual_monitor_fallback": 1,
  "host_privacy": 0
}
```

### config/wavemux-instance.json

WaveMux instance configuration:

```json
{
  "instance": "dev",
  "dataDir": "~/.wavemux-dev"
}
```

## Requirements

- Windows 10/11 Pro or Enterprise
- PowerShell Core (pwsh) 7.0+
- Internet access for downloads
- Administrator rights for some installations
- Parsec account (free tier works)

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Warnings (partial success) |
| 2 | Errors (failed) |
| 3 | Script failure |

## Troubleshooting

### Parsec won't connect without monitor

Enable virtual display in Parsec settings:
1. Settings → Host → "Fallback To Virtual Display" → ON
2. Or use HDMI dummy plug

### WaveMux crashes on startup

Check if correct instance is being used:
```powershell
# Should use --instance=dev
wavemux --instance=dev
```

### Build errors

Rebuild backend after Go changes:
```powershell
task build:backend
```

### Health check fails

Run verbose health check for details:
```powershell
sandbox-health -Verbose
```

## Development

**Location:** `wavemux/tools/sandbox`

**Structure:**
```
sandbox/
├── bin/
│   ├── setup-sandbox-host.ps1  # Setup wrapper
│   └── sandbox-health.ps1      # Health check wrapper
├── scripts/
│   ├── setup-sandbox-impl.ps1  # Main orchestrator
│   ├── install-dev-tools.ps1   # Tool installer
│   ├── install-parsec.ps1      # Parsec setup
│   ├── clone-wavemux.ps1       # WaveMux setup
│   └── sandbox-health-impl.ps1 # Health checks
├── config/
│   ├── parsec-config.json      # Parsec template
│   └── wavemux-instance.json   # Instance config
├── tests/
│   └── Sandbox.Tests.ps1       # Pester tests
├── package.json
└── README.md
```

## License

MIT
