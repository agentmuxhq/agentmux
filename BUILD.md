# Building Wave Terminal

These instructions are for setting up dependencies and building Wave Terminal from source on macOS, Linux, and Windows.

---

## ⚠️ CRITICAL: Version Management

**Before building or releasing, ensure version consistency!**

### Quick Version Bump

```bash
# Bump version (automatically updates all files)
./bump-version.sh patch --message "Your change description"

# Rebuild binaries with new version
task build:backend

# Verify consistency
bash scripts/verify-version.sh
```

**See [README.md](README.md#-version-management--critical---read-this) for full version management guide.**

---

## Prerequisites

### OS-specific dependencies

See [Minimum requirements](README.md#minimum-requirements) to learn whether your OS is supported.

#### macOS

macOS does not have any platform-specific dependencies.

#### Linux

You must have `zip` installed. We also require the [Zig](https://ziglang.org/) compiler for statically linking CGO.

Debian/Ubuntu:

```sh
sudo apt install zip snapd
sudo snap install zig --classic --beta
```

Fedora/RHEL:

```sh
sudo dnf install zip zig
```

Arch:

```sh
sudo pacman -S zip zig
```

##### For packaging

For packaging, the following additional packages are required:

- `fpm` &mdash; If you're on x64 you can skip this. If you're on ARM64, install fpm via [Gem](https://rubygems.org/gems/fpm)
- `rpm` &mdash; If you're not on Fedora, install RPM via your package manager.
- `snapd` &mdash; If your distro doesn't already include it, [install `snapd`](https://snapcraft.io/docs/installing-snapd)
- `lxd` &mdash; [Installation instructions](https://canonical.com/lxd/install)
- `snapcraft` &mdash; Run `sudo snap install snapcraft --classic`
- `libarchive-tools` &mdash; Install via your package manager
- `binutils` &mdash; Install via your package manager
- `libopenjp2-tools` &mdash; Install via your package manager
- `squashfs-tools` &mdash; Install via your package manager

#### Windows

You will need the [Zig](https://ziglang.org/) compiler for statically linking CGO.

You can find installation instructions for Zig on Windows [here](https://ziglang.org/learn/getting-started/#managers).

### Task

Download and install Task (to run the build commands): https://taskfile.dev/installation/

Task is a modern equivalent to GNU Make. We use it to coordinate our build steps. You can find our full Task configuration in [Taskfile.yml](Taskfile.yml).

### Go

Download and install Go via your package manager or directly from the website: https://go.dev/doc/install

### NodeJS

Make sure you have a NodeJS 22 LTS installed.

See NodeJS's website for platform-specific instructions: https://nodejs.org/en/download

We now use `npm`, so you can just run an `npm install` to install node dependencies.

## Clone the Repo

```sh
git clone git@github.com:wavetermdev/waveterm.git
```

or

```sh
git clone https://github.com/wavetermdev/waveterm.git
```

## Install code dependencies

The first time you clone the repo, you'll need to run the following to load the dependencies. If you ever have issues building the app, try running this again:

```sh
task init
```
## Build and Run

All the methods below will install Node and Go dependencies when they run the first time. All these should be run from within the Git repository.

### 🎯 Before You Start

**Important workflow reminders:**
1. ✅ **Development:** Always use `task dev` (hot reload, auto-updates)
2. ❌ **Never** launch from `make/` during development (stale build!)
3. ✅ **Version changes:** Use `./bump-version.sh` then `task build:backend`
4. ✅ **Packaging:** Use `task package` only for final distributables

> [!WARNING]
> **For Development: Use `task dev` Only!**
>
> - During development, **ALWAYS** use `task dev` to run the app
> - **DO NOT** launch WaveTerm from the `make/` directory during development
> - The packaged app in `make/` is **NOT automatically updated** when you make code changes
> - Launching a stale build will cause crashes like "wavesrv.x64.exe ENOENT"
> - Only use `task package` when you need to create a final distributable installer/archive

### Development server

Run the following command to build the app and run it via Vite's development server (this enables Hot Module Reloading):

```sh
task dev
```

**This is the recommended way to run WaveTerm during development.**

### Standalone

Run the following command to build the app and run it standalone, without the development server. This will not reload on change:

```sh
task start
```

### Packaged

Run the following command to generate a production build and package it. This creates distributable installers/archives for installation. All artifacts will be placed in `make/`.

```sh
task package
```

**Note:** This is for creating final packages (installers, zip files, etc.), not for development. After running this, you can install/extract the package to test the production build.

If you're on Linux ARM64, run the following:

```sh
USE_SYSTEM_FPM=1 task package
```

## Debugging

### Frontend logs

You can use the regular Chrome DevTools to debug the frontend application. You can open the DevTools using the keyboard shortcut `Cmd+Option+I` on macOS or `Ctrl+Option+I` on Linux and Windows. Logs will be sent to the Console tab in DevTools.

### Backend logs

Backend logs for the development version of Wave can be found at `~/.waveterm-dev/waveapp.log`. Both the NodeJS backend from Electron and the main Go backend will log here.
