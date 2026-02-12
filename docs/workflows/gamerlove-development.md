# AgentMux Development on Gamerlove

## Quick Start

1. **Connect to gamerlove via Parsec** (or RDP)
2. **Double-click `AgentMux-0.13.0\AgentMux.exe`** on the Desktop
3. AgentMux window opens - fully portable, no dev server needed

---

## Development Mode (Hot Reload)

For active development with hot reload:

```bash
# Start dev server (from container SSH)
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && npx electron-vite dev"
```

**Notes:**
- Use `cd /d` to properly change drives on Windows CMD
- Ignore `docs/tsconfig.json` warnings - they're non-blocking
- Frontend changes auto-reload, Go changes require `task build:backend`

---

## Portable Package Deployment

Each version is deployed as a versioned folder on the desktop:

```
C:\Users\asafe\Desktop\AgentMux-{version}\AgentMux.exe
```

The package is **fully portable** - data is stored in `wave-data\` next to the exe.

---

## Workflow: Agent Deploys New Version

### 1. Build on gamerlove (via SSH)

```bash
# Pull latest code (use cd /d to change drives properly)
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && git fetch origin main && git reset --hard origin/main"

# Build backend
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && task build:backend"

# Build frontend
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && npm run build:prod"
```

### 2. Create portable package

```bash
# Create versioned package directory
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && mkdir make\\AgentMux-{VERSION}"

# Copy Electron framework
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && xcopy /E /Y /I node_modules\\electron\\dist make\\AgentMux-{VERSION}"

# Copy app code
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && mkdir make\\AgentMux-{VERSION}\\resources\\app && xcopy /E /Y /I dist\\main make\\AgentMux-{VERSION}\\resources\\app\\dist\\main && xcopy /E /Y /I dist\\preload make\\AgentMux-{VERSION}\\resources\\app\\dist\\preload && xcopy /E /Y /I dist\\frontend make\\AgentMux-{VERSION}\\resources\\app\\dist\\frontend && copy package.json make\\AgentMux-{VERSION}\\resources\\app\\"

# Copy binaries and schema
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && xcopy /E /Y /I dist\\bin make\\AgentMux-{VERSION}\\bin && xcopy /E /Y /I dist\\schema make\\AgentMux-{VERSION}\\resources\\app\\dist\\schema"

# Rename exe
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && move /Y make\\AgentMux-{VERSION}\\electron.exe make\\AgentMux-{VERSION}\\AgentMux.exe"
```

### 3. Deploy to desktop

```bash
ssh asafe@gamerlove "xcopy /E /Y /I D:\\agentmux-sandbox\\make\\AgentMux-{VERSION} C:\\Users\\asafe\\Desktop\\AgentMux-{VERSION}\\"
```

### 4. Test via Parsec

Connect to gamerlove via Parsec and double-click the new `AgentMux.exe`.

---

## File Locations

| Item | Path |
|------|------|
| Source code | `D:\agentmux-sandbox\` |
| Build artifacts | `D:\agentmux-sandbox\make\` |
| Deployed packages | `C:\Users\asafe\Desktop\AgentMux-{version}\` |
| Portable data | `C:\Users\asafe\Desktop\AgentMux-{version}\wave-data\` |

---

## Troubleshooting

### "Go module cache corrupted"

```bash
ssh asafe@gamerlove "go clean -modcache"
ssh asafe@gamerlove "cd D:/agentmux-sandbox && task build:backend"
```

### "npm peer dependency conflicts"

```bash
ssh asafe@gamerlove "cd D:/agentmux-sandbox && npm install --legacy-peer-deps"
```

### "zod/v4 import error"

```bash
ssh asafe@gamerlove "cd D:/agentmux-sandbox && npm install zod@latest --legacy-peer-deps"
```

---

## Initial Setup (Fresh Clone)

If the sandbox needs to be recreated:

```bash
# Remove old sandbox (backup first if needed)
ssh asafe@gamerlove "powershell -Command \"Remove-Item -Recurse -Force D:\\agentmux-sandbox -ErrorAction SilentlyContinue\""

# Clone fresh (requires PAT - get from secrets)
PAT=$(secrets get services/infra --path gh-admin-pat --raw --no-warning)
ssh asafe@gamerlove "cd /d D:\\ && git clone https://${PAT}@github.com/a5af/agentmux.git agentmux-sandbox"

# Install dependencies
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && npm install --legacy-peer-deps"

# Build backend
ssh asafe@gamerlove "cd /d D:\\agentmux-sandbox && task build:backend"
```

---

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 0.13.0 | 2026-01-02 | Re-cloned sandbox, fixed SSH commands, added dev mode docs |
| 0.13.0 | 2026-01-01 | Single-instance lock removed, first portable deploy |
