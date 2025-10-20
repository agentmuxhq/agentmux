<p align="center">
  <a href="https://www.waveterm.dev">
	<picture>
		<source media="(prefers-color-scheme: dark)" srcset="./assets/wave-dark.png">
		<source media="(prefers-color-scheme: light)" srcset="./assets/wave-light.png">
		<img alt="Wave Terminal Logo" src="./assets/wave-light.png" width="240">
	</picture>
  </a>
  <br/>
</p>

# Wave Terminal - Enhanced Fork

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fwavetermdev%2Fwaveterm.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fwavetermdev%2Fwaveterm?ref=badge_shield)

> **🔱 This is an open fork of [Wave Terminal](https://github.com/wavetermdev/waveterm) with enhanced multi-instance support and improved user experience features.**
>
> **Upstream:** [wavetermdev/waveterm](https://github.com/wavetermdev/waveterm) | **Fork:** [a5af/waveterm](https://github.com/a5af/waveterm)

Wave is an open-source terminal that combines traditional terminal features with graphical capabilities like file previews, web browsing, and AI assistance. It runs on MacOS, Linux, and Windows.

## 🎯 Fork-Specific Features

This fork includes the following enhancements over the upstream Wave Terminal:

### **Multi-Instance Support (Default)**
- **Multiple Wave instances run by default** - no flags needed!
- Each instance automatically gets its own isolated data directory and database
- Shared configuration settings across instances
- Usage examples:
  ```bash
  Wave.exe                        # Auto multi-instance with generated ID
  Wave.exe --instance=test        # Named multi-instance (waveterm-test data directory)
  Wave.exe --instance=v0.12.2     # Named multi-instance (waveterm-v0.12.2 data directory)
  Wave.exe --single-instance      # Enforce only one instance (traditional mode)
  ```

### **Enhanced Instance Management**
- **Multi-instance by default** - launch multiple Wave windows instantly without configuration
- **Optional single-instance mode** - use `--single-instance` flag to enforce one instance only
- **Informative modal dialogs** explaining instance management with clear usage examples
- Prevents silent failures and accidental instance conflicts
- "Learn More" button with direct link to documentation

### **Improved Lock System**
- Robust file lock implementation prevents data corruption
- Separate lock files for each instance (`wave.lock`)
- SQLite database isolation (no WAL file conflicts)
- Graceful error handling with helpful user feedback

### **Better Documentation**
- Comprehensive JSDoc documentation for multi-instance architecture
- Clear path isolation examples
- Usage patterns and best practices

---

## 🔄 Version Management (⚠️ CRITICAL - Read This!)

**Versioning has been a major blocker in the past. This section is ESSENTIAL reading.**

### 📍 Why Version Management Matters

Version consistency across multiple files (package.json, binaries, docs) has caused build failures and deployment issues. **Always use the version bump script** - never edit versions manually.

### ✅ Quick Version Bump (One Command!)

This fork uses **automated version bumping** that updates ALL version areas:

**macOS/Linux (Bash) - RECOMMENDED:**
```bash
./bump-version.sh patch                               # 0.12.10 -> 0.12.11
./bump-version.sh minor --message "Add new feature"   # 0.12.10 -> 0.13.0
./bump-version.sh 0.13.5 --message "Specific version" # Set exact version
```

**Windows (PowerShell):**
```powershell
./bump-version.ps1 patch                              # 0.12.10 -> 0.12.11
./bump-version.ps1 minor -Message "Add new feature"   # 0.12.10 -> 0.13.0
```

### 🔍 What Gets Updated Automatically

The script updates **ALL** of these:
- ✅ `package.json` - Main version source
- ✅ `package-lock.json` - Locked dependencies
- ✅ `VERSION_HISTORY.md` - Fork changelog with date/agent/changes
- ✅ Git commit + tag (e.g., `v0.12.11-fork`)
- ✅ **Verification check** - Ensures consistency across codebase

### ⚙️ After Version Bump - Important!

```bash
# 1. Rebuild backend binaries with new version
task build:backend

# 2. Verify everything is consistent
bash scripts/verify-version.sh

# 3. Push changes
git push origin <branch-name> --tags
```

### 🚨 Common Mistakes to Avoid

❌ **DON'T** manually edit version in package.json
❌ **DON'T** forget to rebuild binaries after version bump
❌ **DON'T** skip version verification
✅ **DO** use bump-version.sh script
✅ **DO** run `task build:backend` after bumping
✅ **DO** check VERSION_HISTORY.md is updated

### 📚 Version Information

- **Current Version:** Check [VERSION_HISTORY.md](./VERSION_HISTORY.md) (always up-to-date)
- **Upstream:** Original [wavetermdev/waveterm](https://github.com/wavetermdev/waveterm) (base: v0.12.0)
- **Fork:** This enhanced [a5af/waveterm](https://github.com/a5af/waveterm) (current: see VERSION_HISTORY.md)

### 🤖 For New Agents

**BEFORE starting work:**
1. Read [VERSION_HISTORY.md](./VERSION_HISTORY.md) - see what's been done
2. Check [CLAUDE.md](./CLAUDE.md) - development workflow
3. Use `bash scripts/verify-version.sh` - verify version consistency

---

Modern development involves constantly switching between terminals and browsers - checking documentation, previewing files, monitoring systems, and using AI tools. Wave brings these graphical tools directly into the terminal, letting you control them from the command line. This means you can stay in your terminal workflow while still having access to the visual interfaces you need.

![WaveTerm Screenshot](./assets/wave-screenshot.webp)

## Key Features

- Flexible drag & drop interface to organize terminal blocks, editors, web browsers, and AI assistants
- Built-in editor for seamlessly editing remote files with syntax highlighting and modern editor features
- Rich file preview system for remote files (markdown, images, video, PDFs, CSVs, directories)
- Integrated AI chat with support for multiple models (OpenAI, Claude, Azure, Perplexity, Ollama)
- Command Blocks for isolating and monitoring individual commands with auto-close options
- One-click remote connections with full terminal and file system access
- Rich customization including tab themes, terminal styles, and background images
- Powerful `wsh` command system for managing your workspace from the CLI and sharing data between terminal sessions

## Installation

Wave Terminal works on macOS, Linux, and Windows.

Platform-specific installation instructions can be found [here](https://docs.waveterm.dev/gettingstarted).

You can also install Wave Terminal directly from: [www.waveterm.dev/download](https://www.waveterm.dev/download).

### Minimum requirements

Wave Terminal runs on the following platforms:

- macOS 11 or later (arm64, x64)
- Windows 10 1809 or later (x64)
- Linux based on glibc-2.28 or later (Debian 10, RHEL 8, Ubuntu 20.04, etc.) (arm64, x64)

The WSH helper runs on the following platforms:

- macOS 11 or later (arm64, x64)
- Windows 10 or later (arm64, x64)
- Linux Kernel 2.6.32 or later (x64), Linux Kernel 3.1 or later (arm64)

## Roadmap

Wave is constantly improving! Our roadmap will be continuously updated with our goals for each release. You can find it [here](./ROADMAP.md).

Want to provide input to our future releases? Connect with us on [Discord](https://discord.gg/XfvZ334gwU) or open a [Feature Request](https://github.com/wavetermdev/waveterm/issues/new/choose)!

## Links

### Upstream Wave Terminal
- Homepage &mdash; https://www.waveterm.dev
- Download Page &mdash; https://www.waveterm.dev/download
- Documentation &mdash; https://docs.waveterm.dev
- Legacy Documentation &mdash; https://legacydocs.waveterm.dev
- Blog &mdash; https://blog.waveterm.dev
- X &mdash; https://x.com/wavetermdev
- Discord Community &mdash; https://discord.gg/XfvZ334gwU

### This Fork
- Fork Repository &mdash; https://github.com/a5af/waveterm
- Fork Issues &mdash; https://github.com/a5af/waveterm/issues
- Upstream Repository &mdash; https://github.com/wavetermdev/waveterm

## Building from Source

See [Building Wave Terminal](BUILD.md).

## 🤖 For AI Agents / Automated Development

**IMPORTANT: Work directly in the main repo at `D:/Code/waveterm`**

Since only one agent works on WaveTerm at a time, there's no need to use worktrees or create separate clones. **Always work in the main repository** to avoid version fragmentation and confusion.

### Before You Start

1. **Check version:** Read [VERSION_HISTORY.md](./VERSION_HISTORY.md) to understand current state
2. **Check branch:** Run `git branch` to see your current branch
3. **Pull latest:** Run `git pull origin <branch-name>` to get latest changes
4. **Read docs:** Review [CLAUDE.md](./CLAUDE.md) for development workflow and critical warnings

### Development Workflow

```bash
# 1. Start development server (required for all code changes)
task dev

# 2. Make your changes (TypeScript/React hot reloads automatically)

# 3. For Go backend changes, rebuild and restart:
task build
# Then kill and restart task dev

# 4. After significant changes, bump version:
./bump-version.sh patch --message "Your change description"
# or on Windows:
./bump-version.ps1 patch -Message "Your change description"

# 5. Push to remote
git push origin <branch-name> --tags
```

### Critical Rules

- ✅ **DO** work in `D:/Code/waveterm` (main repo)
- ✅ **DO** use `task dev` for development
- ✅ **DO** read VERSION_HISTORY.md before starting
- ✅ **DO** use bump-version scripts for version changes
- ❌ **DON'T** create worktrees or additional clones
- ❌ **DON'T** run packaged builds during development (use `task dev`)
- ❌ **DON'T** manually edit version numbers in multiple files

### Quick Reference

- **Main repo:** `D:/Code/waveterm` ← **Work here!**
- **Current version:** Check [VERSION_HISTORY.md](./VERSION_HISTORY.md)
- **Development guide:** [CLAUDE.md](./CLAUDE.md)
- **Build guide:** [BUILD.md](./BUILD.md)
- **Version bump:** `./bump-version.sh` or `./bump-version.ps1`

## Contributing

### Fork-Specific Issues
For issues related to **multi-instance support** or **fork-specific features**, please use this fork's issue tracker:
- **Fork Issues:** https://github.com/a5af/waveterm/issues

### Upstream Contributions
For general Wave Terminal features and bugs, contribute to the upstream repository:
- **Upstream Issues:** https://github.com/wavetermdev/waveterm/issues

Find more information in the upstream [Contributions Guide](CONTRIBUTING.md), which includes:

- [Ways to contribute](CONTRIBUTING.md#contributing-to-wave-terminal)
- [Contribution guidelines](CONTRIBUTING.md#before-you-start)
- [Storybook](https://docs.waveterm.dev/storybook)

### Syncing with Upstream
This fork regularly syncs with upstream Wave Terminal to incorporate new features and bug fixes. Fork-specific enhancements are maintained separately and can be merged or submitted upstream as appropriate.

## License

Wave Terminal is licensed under the Apache-2.0 License. For more information on our dependencies, see [here](./ACKNOWLEDGEMENTS.md).
