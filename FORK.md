# Wave Terminal - Enhanced Fork Documentation

This document describes the enhancements and modifications made in this fork of Wave Terminal.

## Overview

This is an open fork of [Wave Terminal](https://github.com/wavetermdev/waveterm) maintained at [a5af/waveterm](https://github.com/a5af/waveterm). The fork focuses on enhancing multi-instance support and improving user experience when running multiple Wave instances simultaneously.

## Fork-Specific Features

### 1. Multi-Instance Support

**What it does:**
Allows you to run multiple Wave Terminal instances simultaneously without conflicts.

**How to use:**
```bash
# Main instance (no flag)
Wave.exe

# Test instance
Wave.exe --instance=test

# Version-specific instance
Wave.exe --instance=v0.12.2

# Any custom identifier
Wave.exe --instance=my-custom-name
```

**Technical Details:**
- Each instance gets its own isolated data directory
- Data directory pattern: `waveterm-{instance-id}/Data`
- Configuration directory is shared across instances (inherits settings)
- Each instance has its own:
  - `wave.lock` file (prevents conflicts)
  - SQLite databases (`filestore.db`, `waveterm.db`)
  - WAL files (no write-ahead-log conflicts)
  - Process space

**Directory Structure:**
```
Windows:
C:\Users\{user}\AppData\Local\
├── waveterm\Data\                    # Main instance
│   ├── wave.lock
│   └── db\
├── waveterm-test\Data\               # Test instance
│   ├── wave.lock
│   └── db\
└── waveterm-v0.12.2\Data\            # Version instance
    ├── wave.lock
    └── db\

Shared Config:
C:\Users\{user}\.config\waveterm\     # Settings shared by all instances
```

### 2. Enhanced Instance Management

**Informative Modal Dialog:**
When attempting to launch Wave without `--instance` flag while another instance is already running, you'll see a helpful dialog that:
- Explains multi-instance mode
- Shows usage examples
- Provides a "Learn More" button to documentation
- Prevents silent failures

**Backend Error Messages:**
If the backend server (wavesrv) detects a lock conflict, it prints formatted instructions to the log explaining how to use multi-instance mode.

### 3. Improved Lock System

**Robust Implementation:**
- Windows: Uses `alexflint/go-filemutex` for reliable file locking
- POSIX (macOS/Linux): Uses `flock` syscall with `LOCK_EX|LOCK_NB`
- Try-catch error handling prevents crashes
- Graceful degradation with helpful error messages

**Lock File Behavior:**
- Each instance creates `wave.lock` in its data directory
- Lock is held for the lifetime of the wavesrv process
- Automatically released when instance exits
- Non-blocking lock attempts (fails fast if already locked)

### 4. Enhanced Documentation

**In-Code Documentation:**
- Comprehensive JSDoc comments for multi-instance functions
- Clear examples in function documentation
- Architecture explanations in code comments

**User Documentation:**
- Updated README with fork-specific features
- This FORK.md document
- Usage examples throughout

## Use Cases

### Testing New Versions
```bash
# Keep your stable version running
Wave.exe

# Test a new portable version side-by-side
Wave-v0.12.2.exe --instance=test-v0.12.2
```

### Development Workflows
```bash
# Production environment
Wave.exe --instance=prod

# Staging environment
Wave.exe --instance=staging

# Development environment
Wave.exe --instance=dev
```

### Multiple Projects
```bash
# Different projects with isolated state
Wave.exe --instance=project-alpha
Wave.exe --instance=project-beta
```

## Differences from Upstream

| Feature | Upstream | This Fork |
|---------|----------|-----------|
| Multiple instances | ❌ Not supported | ✅ Full support with `--instance` flag |
| Lock conflict handling | Silent exit | Informative modal dialog |
| Error messages | Technical errors | User-friendly explanations |
| Data isolation | N/A | Complete isolation per instance |
| Config sharing | N/A | Shared across instances |
| Documentation | Standard | Enhanced with examples |

## Migration from Upstream

If you're coming from upstream Wave Terminal:

1. **Your existing data is safe:** The main instance (no `--instance` flag) uses the same data directory as before
2. **Settings are preserved:** Configuration remains in the same location
3. **No breaking changes:** All upstream features work exactly as before
4. **New capability:** You can now run multiple instances if needed

## Contributing to This Fork

### Reporting Fork-Specific Issues
Use this fork's issue tracker for:
- Multi-instance support issues
- Lock system problems
- Fork-specific feature requests

**Issue Tracker:** https://github.com/a5af/waveterm/issues

### Reporting Upstream Issues
For general Wave Terminal bugs and features, contribute to:

**Upstream Tracker:** https://github.com/wavetermdev/waveterm/issues

## Syncing with Upstream

This fork regularly syncs with upstream to incorporate:
- New features
- Bug fixes
- Security updates
- Performance improvements

Fork-specific enhancements are maintained in separate commits and can be:
- Merged when upstream adds similar features
- Submitted as PRs to upstream if appropriate
- Kept as fork-specific if they serve a niche use case

## Version Numbering

This fork follows the upstream version numbering with minor version increments for fork-specific releases:
- Upstream: `v0.12.0`
- Fork: `v0.12.1` (adds multi-instance support)
- Fork: `v0.12.2` (improves modal dialog)

## License

This fork maintains the same Apache-2.0 License as upstream Wave Terminal.

## Credits

- **Upstream Wave Terminal:** [wavetermdev/waveterm](https://github.com/wavetermdev/waveterm)
- **Fork Maintainer:** [a5af](https://github.com/a5af)
- **Contributors:** See [Contributors](https://github.com/a5af/waveterm/graphs/contributors)

## Support

For help with this fork:
1. Check this documentation
2. Review existing issues: https://github.com/a5af/waveterm/issues
3. Open a new issue if needed

For general Wave Terminal support:
- Documentation: https://docs.waveterm.dev
- Discord: https://discord.gg/XfvZ334gwU
