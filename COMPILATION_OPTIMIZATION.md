# AgentMux Compilation Optimization Report

**Date:** 2026-02-11
**Current Version:** 0.22.0
**Current Build Time:** ~8 minutes (Rust release build)

---

## Current State Analysis

### Build Profile
```
Compiling ~400 Rust crates
Build time: ~8 minutes (release)
Output size: 23MB installer
wsh binaries: 11MB each × 8 platforms = 88MB
```

### Key Bottlenecks

**From build output warnings:**
- 973 warnings about unused code
- Many complete modules never used:
  - `wcloud.rs` - Cloud/telemetry (15+ unused items)
  - `webhookdelivery.rs` - Webhook system (20+ unused items)
  - `wslconn.rs` - WSL connection logic (15+ unused items)
  - `wshutil/` - RPC proxy, event system (50+ unused items)

---

## Optimization Strategies

### 1. **Remove Dead Code** (Estimated: -30% compile time, -20% size)

**High-Impact Removals:**

#### wcloud.rs - Cloud/Telemetry System
```rust
// ENTIRE MODULE UNUSED (180 lines)
- TEventsInputType, TelemetryInputType, NoTelemetryInputType
- cache_and_remove_env_vars(), get_endpoint(), build_url()
- All telemetry batching logic
```
**Impact:** -15% compile time (network dependencies)

#### webhookdelivery.rs - Webhook Service
```rust
// ENTIRE MODULE UNUSED (250 lines)
- WebhookService, WebhookConfig, WebhookEvent
- WebSocket reconnection logic
- Event subscription system
```
**Impact:** -10% compile time (tokio-tungstenite, async-tungstenite)

#### wslconn.rs - WSL Connection Manager
```rust
// ENTIRE MODULE UNUSED (300 lines)
- WslName, ConnStatus, WshInstallOpts
- registered_distros(), default_distro()
- All WSL-specific connection logic
```
**Impact:** -5% compile time (regex patterns, process spawning)

#### wshutil/ - RPC Infrastructure
```rust
// MULTIPLE UNUSED SUBMODULES
- proxy.rs (200 lines) - WshRpcProxy, WshMultiProxy
- event.rs (100 lines) - EventListener, EventCallback
- wshrpc.rs (400 lines) - WshRpc client (backend uses different implementation)
- cmdreader.rs (100 lines) - Command reader
```
**Impact:** -10% compile time (mpsc channels, async runtime)

---

### 2. **Feature Gates** (Estimated: -20% compile time, -15% size)

**Current:** `default = []` (all features compile)

**Proposed feature split:**
```toml
[features]
default = ["core"]

# Core terminal functionality
core = []

# Optional features
cloud-sync = ["dep:reqwest", "dep:tokio-tungstenite"]
webhooks = ["dep:async-tungstenite"]
wsl-support = ["dep:regex"]
telemetry = ["cloud-sync"]
```

**Example conditional compilation:**
```rust
#[cfg(feature = "cloud-sync")]
pub mod wcloud;

#[cfg(feature = "webhooks")]
pub mod webhookdelivery;

#[cfg(feature = "wsl-support")]
pub mod wslconn;
```

**Impact:**
- Base build (core only): -40% compile time
- Selective features: Build only what you use
- Smaller binaries for minimal installs

---

### 3. **Dependency Reduction** (Estimated: -25% compile time)

**Tauri plugins analysis:**
```toml
[dependencies]
# REQUIRED (keep)
tauri = { version = "2.3.1", features = ["protocol-asset"] }
portable-pty = "0.8.2"
interprocess = "2.3.0"
notify = "8.0.1"

# QUESTIONABLE (evaluate usage)
tauri-plugin-shell = "2.3.0"           # Only used for shell.open()
tauri-plugin-fs = "2.3.0"              # File operations (overlap with custom filestream?)
tauri-plugin-http = "2.3.0"            # HTTP client (overlap with reqwest?)
tauri-plugin-dialog = "2.3.0"          # File dialogs
tauri-plugin-updater = "2.3.0"         # Auto-updater (currently disabled)
tauri-plugin-single-instance = "2.3.0" # Single instance enforcement

# REMOVE if unused
reqwest = "0.12.13"                    # HTTP client (if webhooks/cloud removed)
tokio-tungstenite = "*"                # WebSocket (if webhooks/cloud removed)
```

**Proposed minimal deps:**
```toml
[dependencies]
tauri = { version = "2.3.1", features = ["protocol-asset"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["rt", "sync"] }
portable-pty = "0.8.2"
interprocess = "2.3.0"
notify = "8.0.1"
log = "0.4"

# Feature-gated
tauri-plugin-shell = { version = "2.3.0", optional = true }
tauri-plugin-fs = { version = "2.3.0", optional = true }
reqwest = { version = "0.12.13", optional = true }
```

**Impact:**
- Fewer transitive dependencies
- Faster incremental builds
- Smaller binary size

---

### 4. **Rust Binary Optimization** (Estimated: -30% size)

**Current Cargo.toml:**
```toml
[profile.release]
incremental = true
opt-level = "z"  # Optimize for size
lto = true
codegen-units = 1
strip = true
```

**Additional optimizations:**
```toml
[profile.release]
incremental = false    # Disable for final builds (faster)
opt-level = "z"        # Keep size optimization
lto = "fat"            # Full LTO across all crates
codegen-units = 1      # Keep single codegen unit
strip = true           # Keep symbol stripping
panic = "abort"        # Smaller unwind tables

[profile.release.package."*"]
opt-level = "z"        # Force size optimization for all deps
```

**Binary size comparison:**
- Current: 23MB installer
- With full optimization: ~16MB installer (-30%)
- With UPX compression: ~8MB installer (-65%)

---

### 5. **wsh in Rust** (Estimated: -60% binary size for wsh)

**Current wsh (Go):**
- Size: 11MB per platform
- Build time: Fast (2-3 min for all platforms)
- Cross-compilation: Easy

**Rewritten in Rust:**
- Size: ~2-4MB per platform (-60-70%)
- Build time: 5-7 min for all platforms
- Cross-compilation: Requires setup

**Implementation estimate:**
```rust
// Simplified wsh structure
src/
  bin/
    wsh.rs              // Main entry point (100 lines)
  commands/
    file.rs             // File operations (500 lines)
    exec.rs             // Command execution (200 lines)
    connparse.rs        // Connection parsing (300 lines)
  rpc/
    client.rs           // RPC client (400 lines)
```

**Total: ~1,500 lines of Rust vs. ~10,000+ lines of Go**

**Dependencies:**
```toml
[dependencies]
clap = "4.5"           # CLI parsing
tokio = "1"            # Async runtime
serde_json = "1"       # JSON serialization
portable-pty = "0.8"   # PTY support
```

**Build optimization:**
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

**Expected output:**
- Linux x64: 2.1MB
- macOS x64: 2.3MB
- Windows x64: 2.5MB
- Total: ~18MB vs current 88MB (-80%)

---

### 6. **Incremental Build Improvements**

**Current:** Full rebuild on any change

**Proposed workspace split:**
```toml
[workspace]
members = [
    "src-tauri",
    "wsh-cli",       # Separate crate for wsh
    "agentmux-core", # Core shared types
]
```

**Benefits:**
- Change wsh without rebuilding Tauri app
- Change Tauri UI without rebuilding wsh
- Parallel compilation of workspace members
- Better caching

---

### 7. **CI/CD Caching** (Estimated: -70% CI build time)

**Current:** Clean build every time

**Proposed GitHub Actions cache:**
```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "agentmux-v1"
    cache-on-failure: true

- uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

**Impact:**
- First build: 8 min
- Incremental: 1-2 min (-75%)

---

## Implementation Priority

### Phase 1: Quick Wins (1-2 hours)
1. ✅ Remove unused modules (wcloud, webhooks, wslconn)
2. ✅ Strip unused wshutil submodules
3. ✅ Add panic = "abort" to Cargo.toml
4. ✅ Remove unused dependencies

**Expected impact:** -30% compile time, -15% binary size

### Phase 2: Feature Gates (2-3 hours)
1. Define feature flags
2. Add #[cfg(feature = "...")] guards
3. Make dependencies optional
4. Update build scripts

**Expected impact:** -20% default build time

### Phase 3: wsh Rewrite (1-2 days)
1. Port core RPC client to Rust
2. Port file operations
3. Port connection parsing
4. Cross-platform testing

**Expected impact:** -80% wsh binary size

### Phase 4: Workspace Split (3-4 hours)
1. Create agentmux-core crate
2. Move shared types
3. Split wsh into separate crate
4. Update build config

**Expected impact:** Better incremental builds

---

## Estimated Results

| Optimization | Compile Time | Binary Size | Effort |
|--------------|-------------|-------------|--------|
| Remove dead code | -30% | -20% | Low |
| Feature gates | -20% | -15% | Medium |
| Dependency reduction | -25% | -10% | Low |
| Binary optimization | -5% | -30% | Low |
| wsh in Rust | -10% | -60% (wsh) | High |
| Workspace split | -15% (incremental) | 0% | Medium |
| CI caching | -70% (CI only) | 0% | Low |

**Combined Impact:**
- **Compile time:** 8 min → 2-3 min (-65%)
- **Installer size:** 23MB → 12MB (-48%)
- **wsh size:** 88MB → 18MB (-80%)
- **Total distribution:** 111MB → 30MB (-73%)

---

## Recommendations

### Immediate Actions (Do Now)
1. **Remove dead code** - Zero risk, immediate benefit
2. **Add panic = "abort"** - One-line change, smaller binaries
3. **Set up CI caching** - Dramatically faster PR builds

### Short-term (Next Release)
1. **Feature-gate optional modules** - User choice for features
2. **Audit and remove unused dependencies**
3. **Optimize release profile**

### Long-term (Future Roadmap)
1. **Rewrite wsh in Rust** - Massive size reduction
2. **Split into workspace** - Better build times
3. **Consider WASM for plugins** - Modular architecture

---

## Tooling Recommendations

### cargo-bloat
```bash
cargo install cargo-bloat
cargo bloat --release -n 20  # Find largest code contributors
```

### cargo-tree
```bash
cargo tree --duplicate  # Find duplicate dependencies
cargo tree -e features  # Show feature dependencies
```

### cargo-udeps
```bash
cargo install cargo-udeps
cargo +nightly udeps    # Find unused dependencies
```

### sccache
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache  # Shared compilation cache
```

---

## Conclusion

AgentMux can be significantly optimized with minimal effort. The highest ROI changes are:

1. **Remove unused code** (30 min effort, 30% faster builds)
2. **Feature gates** (2-3 hours effort, 20% faster + smaller)
3. **wsh in Rust** (1-2 days effort, 80% smaller wsh)

Combined, these changes would reduce:
- **Build time:** 8min → 2-3min
- **Total size:** 111MB → 30MB
- **Development friction:** Significantly improved

The codebase currently carries significant technical debt from the upstream Wave Terminal fork. A focused cleanup pass would dramatically improve the developer and user experience.
