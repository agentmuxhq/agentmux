# Building WaveMux

These instructions are for setting up dependencies and building WaveMux from source on Windows, macOS, and Linux.

**Architecture:** WaveMux is built on **Tauri v2** (not Electron), with a Go backend sidecar.

---

## Prerequisites

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| **Node.js** | v22 LTS | Frontend build (React/Vite) |
| **Go** | 1.23+ | Backend (wavemuxsrv, wsh) |
| **Rust** | 1.77+ | Tauri frontend (Rust) |
| **Task** | Latest | Build orchestration |
| **Zig** | 0.13+ | CGO cross-compilation (Windows/Linux) |

### Platform-Specific Setup

#### Windows

1. **Install Zig** (for CGO):
   ```powershell
   # Download from https://ziglang.org/download/
   # Extract to C:\Systems\zig-windows-x86_64-0.13.0\
   # Add to PATH
   ```

2. **Install Rust** (for Tauri):
   ```powershell
   # Download from https://rustup.rs/
   rustup-init.exe
   ```

3. **Install Visual Studio Build Tools** (required by Rust):
   - Download: https://visualstudio.microsoft.com/visual-cpp-build-tools/
   - Install: "Desktop development with C++"

#### macOS

1. **Install Xcode Command Line Tools**:
   ```bash
   xcode-select --install
   ```

2. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **No Zig required** (macOS CGO works natively)

#### Linux

1. **Install dependencies** (Debian/Ubuntu):
   ```bash
   sudo apt install zip libwebkit2gtk-4.1-dev \
     build-essential curl wget file libssl-dev \
     libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

   # Install Zig for CGO
   sudo snap install zig --classic --beta
   ```

2. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

### Install Task

Task is our build orchestration tool (modern equivalent to GNU Make):

```bash
# macOS
brew install go-task/tap/go-task

# Linux
sudo snap install task --classic

# Windows (PowerShell)
winget install Task.Task
```

See full instructions: https://taskfile.dev/installation/

---

## Clone the Repository

```bash
git clone https://github.com/a5af/wavemux.git
cd wavemux
```

---

## Install Dependencies

First time setup (run this after cloning):

```bash
# Install Node and Go dependencies
task init
```

If you have build issues later, run `task init` again to refresh dependencies.

---

## Build Commands

### Development (Hot Reload)

**This is the recommended way to run WaveMux during development:**

```bash
task dev
```

Features:
- ✅ Frontend hot reload (React HMR)
- ✅ Tauri auto-rebuild on Rust changes
- ✅ Backend auto-restart on crash
- ✅ DevTools available (Ctrl+Shift+I)

**Important:** Always use `task dev` for development. Never launch from `src-tauri/target/` directly.

---

### Backend Rebuild

If you modify Go backend code (`pkg/`, `cmd/`):

```bash
# Rebuild Go binaries
task build:backend

# Then restart dev server
task dev
```

This rebuilds:
- `dist/bin/wavemuxsrv.x64.exe` (or platform-specific)
- `dist/bin/wsh-{version}-{platform}.{arch}.exe`

---

### Production Build

Create a production Tauri build with installer:

```bash
task build
```

Output locations:
- **Windows:** `src-tauri/target/release/bundle/nsis/WaveMux_{version}_x64-setup.exe`
- **macOS:** `src-tauri/target/release/bundle/dmg/WaveMux_{version}_x64.dmg`
- **Linux:** `src-tauri/target/release/bundle/deb/wavemux_{version}_amd64.deb`

**Note:** This creates final installers for distribution, not for development.

---

## Version Management

**Before releasing, ensure version consistency across all files:**

```bash
# Bump version (updates package.json, Cargo.toml, tauri.conf.json, etc.)
./bump-version.sh patch --message "Your change description"

# Rebuild backend with new version
task build:backend

# Verify consistency
bash scripts/verify-version.sh

# Push with tags
git push origin <branch> --tags
```

**Critical:** Always use `bump-version.sh` - never manually edit version numbers.

---

## Development Workflow

### Typical Development Session

```bash
# 1. Pull latest changes
git checkout main
git pull origin main

# 2. Create feature branch
git checkout -b agenta/feature-name

# 3. Start dev server
task dev

# 4. Make changes to code
# - Frontend (frontend/): Auto-reloads
# - Tauri (src-tauri/src/): Auto-rebuilds
# - Backend (pkg/, cmd/): Run `task build:backend`, restart dev

# 5. Test changes in running app

# 6. Commit and push
git commit -m "feat: description"
git push -u origin agenta/feature-name

# 7. Create PR
gh pr create --title "Feature" --body "Description"
```

---

## Architecture

### Build Output

After building, you'll have:

```
dist/
├── bin/
│   ├── wavemuxsrv.x64.exe       # Go backend (33MB)
│   └── wsh-{version}-windows.x64.exe # Shell integration (11MB)
└── frontend/                     # Vite output (embedded in Tauri)

src-tauri/target/release/
├── wavemux.exe                   # Tauri app (14MB)
└── bundle/
    └── nsis/
        └── WaveMux_{version}_x64-setup.exe  # Installer (29MB)
```

### Component Sizes

| Component | Size | Purpose |
|-----------|------|---------|
| `wavemux.exe` | ~14MB | Tauri frontend (Rust + webview) |
| `wavemuxsrv.exe` | ~33MB | Go backend server |
| `wsh.exe` | ~11MB | Shell integration |
| **Total runtime** | ~58MB | All components |
| **Installer (NSIS)** | ~29MB | Compressed with binaries |

Compare to Electron version: ~135MB runtime, ~120-150MB installer.

---

## Debugging

### Frontend Logs

Open Chrome DevTools in the app:
- **Windows/Linux:** `Ctrl+Shift+I`
- **macOS:** `Cmd+Option+I`

Logs appear in the Console tab.

### Backend Logs

Go backend logs (wavemuxsrv):
- **Development:** `~/.waveterm-dev/waveapp.log`
- **Production:** `~/.waveterm/waveapp.log`

View logs in real-time:

```bash
# Development
tail -f ~/.waveterm-dev/waveapp.log

# Production
tail -f ~/.waveterm/waveapp.log
```

### Tauri Logs

Rust logs appear in the terminal where you ran `task dev`.

---

## Troubleshooting

### Issue: "wavemuxsrv.x64.exe ENOENT"

**Cause:** Backend binary not found or wrong version.

**Fix:**
```bash
# Check version mismatch
cat package.json | grep version
ls -lh dist/bin/wsh-*

# Rebuild backend
task build:backend
```

### Issue: Tauri build fails with linker errors

**Cause:** Missing Rust toolchain or system libraries.

**Fix (Windows):**
```powershell
# Install Visual Studio Build Tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

**Fix (Linux):**
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev
```

### Issue: CGO errors during Go build

**Cause:** Zig compiler not installed or not in PATH.

**Fix:**
```bash
# Verify Zig is installed
zig version

# Add to PATH (example for Windows)
$env:PATH += ";C:\Systems\zig-windows-x86_64-0.13.0"
```

### Issue: Frontend not loading in dev mode

**Cause:** Vite dev server failed to start.

**Fix:**
```bash
# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm install
task dev
```

---

## CI/CD

### GitHub Actions

Automated builds run on every push to `main`:

- **Windows:** NSIS installer (.exe)
- **macOS:** DMG installer (.dmg)
- **Linux:** DEB package (.deb), AppImage

Artifacts are uploaded to GitHub Releases on tagged commits.

### Local Release Build

To create a release build locally:

```bash
# 1. Bump version
./bump-version.sh minor --message "v0.19.0 release"

# 2. Rebuild backend
task build:backend

# 3. Build Tauri package
task build

# 4. Test installer
# Install from src-tauri/target/release/bundle/

# 5. Tag and push
git push origin main --tags
```

---

## Cross-Platform Notes

### Windows

- Uses **NSIS** for installers
- WebView2 runtime required (auto-installs if missing)
- Zig required for CGO (Go → C compilation)

### macOS

- Uses **DMG** for distribution
- WKWebView built-in (no WebView2 needed)
- Code signing required for distribution (not dev)
- Universal binary supported (x64 + ARM64)

### Linux

- Uses **DEB** (Debian/Ubuntu) and **AppImage** (universal)
- WebKitGTK required: `libwebkit2gtk-4.1-dev`
- Different distros may need different dependencies

---

## Advanced: Custom Build

### Build Backend for Specific Platform

```bash
# Windows x64
GOOS=windows GOARCH=amd64 task build:server

# macOS ARM64
GOOS=darwin GOARCH=arm64 task build:server

# Linux x64
GOOS=linux GOARCH=amd64 task build:server
```

### Build Frontend Only

```bash
# Development build
npm run build:dev

# Production build
npm run build:prod
```

### Build Tauri Only (no backend rebuild)

```bash
npm run tauri build
```

---

## Resources

- **Tauri Documentation:** https://tauri.app/v2/
- **Task Configuration:** [Taskfile.yml](Taskfile.yml)
- **Architecture Docs:** [docs/architecture/wavemux-components.md](docs/architecture/wavemux-components.md)
- **Version Management:** [README.md](README.md)
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md)

---

## Summary

**Quick Reference:**

| Task | Command |
|------|---------|
| **Development** | `task dev` |
| **Rebuild backend** | `task build:backend` |
| **Production build** | `task build` |
| **Bump version** | `./bump-version.sh patch` |
| **Run tests** | `npm test` |

**Remember:** Always use `task dev` for development, never launch stale builds from `make/` or `target/`!
