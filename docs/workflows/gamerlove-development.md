# WaveMux Development on Gamerlove

## Quick Start

1. **Connect to gamerlove via Parsec** (or RDP)
2. **Double-click `WaveMux-0.13.0\WaveMux.exe`** on the Desktop
3. WaveMux window opens - fully portable, no dev server needed

---

## Portable Package Deployment

Each version is deployed as a versioned folder on the desktop:

```
C:\Users\asafe\Desktop\WaveMux-{version}\WaveMux.exe
```

The package is **fully portable** - data is stored in `wave-data\` next to the exe.

---

## Workflow: Agent Deploys New Version

### 1. Build on gamerlove (via SSH)

```bash
# Pull latest code
ssh asafe@gamerlove "cd D:/wavemux-sandbox && git fetch origin main && git reset --hard origin/main"

# Build backend
ssh asafe@gamerlove "cd D:/wavemux-sandbox && task build:backend"

# Build frontend
ssh asafe@gamerlove "cd D:/wavemux-sandbox && npm run build:prod"
```

### 2. Create portable package

```bash
# Create versioned package directory
ssh asafe@gamerlove "D: && cd D:\\wavemux-sandbox && mkdir make\\WaveMux-{VERSION}"

# Copy Electron framework
ssh asafe@gamerlove "xcopy /E /Y /I node_modules\\electron\\dist make\\WaveMux-{VERSION}"

# Copy app code
ssh asafe@gamerlove "mkdir make\\WaveMux-{VERSION}\\resources\\app && xcopy /E /Y /I dist\\main make\\WaveMux-{VERSION}\\resources\\app\\dist\\main && xcopy /E /Y /I dist\\preload make\\WaveMux-{VERSION}\\resources\\app\\dist\\preload && xcopy /E /Y /I dist\\frontend make\\WaveMux-{VERSION}\\resources\\app\\dist\\frontend && copy package.json make\\WaveMux-{VERSION}\\resources\\app\\"

# Copy binaries and schema
ssh asafe@gamerlove "xcopy /E /Y /I dist\\bin make\\WaveMux-{VERSION}\\bin && xcopy /E /Y /I dist\\schema make\\WaveMux-{VERSION}\\resources\\app\\dist\\schema"

# Rename exe
ssh asafe@gamerlove "move /Y make\\WaveMux-{VERSION}\\electron.exe make\\WaveMux-{VERSION}\\WaveMux.exe"
```

### 3. Deploy to desktop

```bash
ssh asafe@gamerlove "xcopy /E /Y /I D:\\wavemux-sandbox\\make\\WaveMux-{VERSION} C:\\Users\\asafe\\Desktop\\WaveMux-{VERSION}"
```

### 4. Test via Parsec

Connect to gamerlove via Parsec and double-click the new `WaveMux.exe`.

---

## File Locations

| Item | Path |
|------|------|
| Source code | `D:\wavemux-sandbox\` |
| Build artifacts | `D:\wavemux-sandbox\make\` |
| Deployed packages | `C:\Users\asafe\Desktop\WaveMux-{version}\` |
| Portable data | `C:\Users\asafe\Desktop\WaveMux-{version}\wave-data\` |

---

## Troubleshooting

### "Go module cache corrupted"

```bash
ssh asafe@gamerlove "go clean -modcache"
ssh asafe@gamerlove "cd D:/wavemux-sandbox && task build:backend"
```

### "npm peer dependency conflicts"

```bash
ssh asafe@gamerlove "cd D:/wavemux-sandbox && npm install --legacy-peer-deps"
```

### "zod/v4 import error"

```bash
ssh asafe@gamerlove "cd D:/wavemux-sandbox && npm install zod@latest --legacy-peer-deps"
```

---

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 0.13.0 | 2026-01-01 | Single-instance lock removed, first portable deploy |
