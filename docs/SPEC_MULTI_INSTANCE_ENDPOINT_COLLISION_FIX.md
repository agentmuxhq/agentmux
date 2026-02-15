# Multi-Instance Endpoint File Collision Bug

**Version:** 0.27.11
**Date:** 2026-02-15
**Severity:** P0 - Critical
**Status:** Identified

---

## Problem Summary

Opening multiple AgentMux instances causes input blocking in previously opened instances. Each new instance "steals" the frontend connections from earlier instances by overwriting a shared endpoint configuration file.

## Solution Summary

**Use nested instance directories:**
- All instance data inside `com.a5af.agentmux` folder (no separate `-instance-1` folders)
- Each instance gets subfolder: `instances/default/`, `instances/instance-1/`, etc.
- Track backend version in endpoints file
- Verify version before reusing backend
- Automatic migration from old flat structure

**Directory structure:**
```
com.a5af.agentmux\
  instances\
    default\          ← Instance 1
      wave-endpoints.json (with version)
    instance-1\       ← Instance 2
      wave-endpoints.json (with version)
    instance-2\       ← Instance 3
      wave-endpoints.json (with version)
```

---

## User-Reported Symptoms

1. **2 instances open:**
   - First instance stops accepting keyboard input
   - New terminal tabs in first instance show blank panes
   - Second instance works normally

2. **3 instances open:**
   - Third instance displays input that was in second instance
   - Second instance stops accepting input
   - First instance remains broken

3. **Pattern:** Each new instance breaks the previous instance in a cascading failure.

---

## Root Cause Analysis

### Backend (Go) - Working Correctly ✅

The backend multi-instance system works as designed:

1. **Instance 1 starts:**
   ```
   Lock:   %LOCALAPPDATA%\com.a5af.agentmux\wave-0.27.11.lock
   Socket: %LOCALAPPDATA%\com.a5af.agentmux\wave-0.27.11.sock
   DB:     %LOCALAPPDATA%\com.a5af.agentmux\db\
   ```

2. **Instance 2 starts:**
   - Default lock already held → tries `instance-1`
   - Acquires lock at: `%LOCALAPPDATA%\com.a5af.agentmux-instance-1\wave-0.27.11.lock`
   - Updates `DataHome_VarCache` to instance-specific directory
   ```
   Lock:   %LOCALAPPDATA%\com.a5af.agentmux-instance-1\wave-0.27.11.lock
   Socket: %LOCALAPPDATA%\com.a5af.agentmux-instance-1\wave-0.27.11.sock
   DB:     %LOCALAPPDATA%\com.a5af.agentmux-instance-1\db\
   ```

3. **Instance 3 starts:**
   ```
   Lock:   %LOCALAPPDATA%\com.a5af.agentmux-instance-2\wave-0.27.11.lock
   Socket: %LOCALAPPDATA%\com.a5af.agentmux-instance-2\wave-0.27.11.sock
   DB:     %LOCALAPPDATA%\com.a5af.agentmux-instance-2\db\
   ```

**Backend isolation:** ✅ Each instance has separate lock, socket, and database.

### Frontend (Tauri/Rust) - Broken ❌

The frontend uses **shared directories** for all instances:

**File:** `src-tauri/src/sidecar.rs`

```rust
// Lines 25-36: SAME for all instances
let data_dir = app.path().app_data_dir()?;      // %LOCALAPPDATA%\com.a5af.agentmux
let config_dir = app.path().app_config_dir()?;  // %APPDATA%\com.a5af.agentmux

// Lines 133-134: All instances get SAME env vars
.env("WAVETERM_DATA_HOME", data_dir.to_string_lossy().to_string())
.env("WAVETERM_CONFIG_HOME", config_dir.to_string_lossy().to_string())
```

**The collision:**

```rust
// Line 39: SHARED FILE across all instances
let endpoints_file = config_dir.join("wave-endpoints.json");
// %APPDATA%\com.a5af.agentmux\wave-endpoints.json

// Lines 224-232: Each instance OVERWRITES this file
std::fs::write(&endpoints_file, &json)?;
```

---

## Failure Sequence

### Instance 1 Starts

1. Frontend spawns backend with `WAVETERM_DATA_HOME=%LOCALAPPDATA%\com.a5af.agentmux`
2. Backend acquires default lock, creates socket at `wave-0.27.11.sock`
3. Backend prints: `WAVESRV-ESTART ws:127.0.0.1:PORT1 web:127.0.0.1:PORT2`
4. Frontend saves to `%APPDATA%\com.a5af.agentmux\wave-endpoints.json`:
   ```json
   {
     "ws_endpoint": "127.0.0.1:PORT1",
     "web_endpoint": "127.0.0.1:PORT2",
     "auth_key": "instance1-key"
   }
   ```
5. Frontend connects to ws:PORT1, web:PORT2 ✅

### Instance 2 Starts

1. Frontend spawns backend with **SAME** `WAVETERM_DATA_HOME`
2. Backend fails default lock → acquires `instance-1` lock
3. Backend updates DataHome_VarCache to instance-1 directory
4. Backend creates socket at `wave-0.27.11.sock` in **instance-1** directory (isolated ✅)
5. Backend prints: `WAVESRV-ESTART ws:127.0.0.1:PORT3 web:127.0.0.1:PORT4`
6. Frontend **OVERWRITES** `%APPDATA%\com.a5af.agentmux\wave-endpoints.json`:
   ```json
   {
     "ws_endpoint": "127.0.0.1:PORT3",
     "web_endpoint": "127.0.0.1:PORT4",
     "auth_key": "instance2-key"
   }
   ```
7. Frontend2 connects to ws:PORT3, web:PORT4 ✅

**Instance 1 is now BROKEN:**
- Its `wave-endpoints.json` file was overwritten
- If frontend1 reconnects, it will read PORT3/PORT4 (instance 2's endpoints)
- Frontend1 will connect to Backend2
- Backend1 is orphaned with no frontend

### Instance 3 Starts

1. Same process - backend acquires `instance-2` lock
2. Frontend **OVERWRITES** endpoints file AGAIN:
   ```json
   {
     "ws_endpoint": "127.0.0.1:PORT5",
     "web_endpoint": "127.0.0.1:PORT6",
     "auth_key": "instance3-key"
   }
   ```

**Now:**
- Instance 1 frontend → connects to instance 2 backend (if reconnects)
- Instance 2 frontend → connects to instance 3 backend (if reconnects)
- Instance 3 frontend → connects to instance 3 backend ✅
- Backends 1 and 2 are orphaned

---

## Why Input Breaks

When a frontend connects to the wrong backend:

1. **Terminal input goes to wrong backend**
   - User types in instance 1 window
   - Frontend1 sends keystrokes to backend2 (wrong backend)
   - Backend2's terminals receive the input
   - Instance 2 window shows the input

2. **New tabs show blank panes**
   - User opens new tab in instance 1
   - Frontend1 asks backend2 to create terminal
   - Backend2 creates terminal in its own database
   - Backend1 never sees the request
   - Instance 1 window shows nothing (wrong backend, wrong DB)

3. **Why 3rd instance "steals" from 2nd**
   - Opening instance 3 overwrites endpoints file
   - Instance 2 frontend loses connection or reconnects
   - Reads new endpoints file
   - Connects to backend3
   - Now typing in instance 2 appears in instance 3

---

## Technical Details

### File Locations (Windows)

**Current (Broken):**
```
Default Instance:
  DATA:   C:\Users\<user>\AppData\Local\com.a5af.agentmux\
  CONFIG: C:\Users\<user>\AppData\Roaming\com.a5af.agentmux\
  ENDPOINTS: wave-endpoints.json  ❌

Instance 2:
  DATA:   C:\Users\<user>\AppData\Local\com.a5af.agentmux-instance-1\
  CONFIG: C:\Users\<user>\AppData\Roaming\com.a5af.agentmux\
  ENDPOINTS: wave-endpoints.json  ❌ SAME FILE!

Instance 3:
  DATA:   C:\Users\<user>\AppData\Local\com.a5af.agentmux-instance-2\
  CONFIG: C:\Users\<user>\AppData\Roaming\com.a5af.agentmux\
  ENDPOINTS: wave-endpoints.json  ❌ SAME FILE!
```

**The bug:** All instances share `%APPDATA%\com.a5af.agentmux\wave-endpoints.json`

**Desired (Nested Structure):**
```
C:\Users\<user>\AppData\Local\com.a5af.agentmux\
  instances\
    default\
      wave-0.27.11.lock
      wave-0.27.11.sock
      db\
    instance-1\
      wave-0.27.11.lock
      wave-0.27.11.sock
      db\
    instance-2\
      wave-0.27.11.lock
      wave-0.27.11.sock
      db\

C:\Users\<user>\AppData\Roaming\com.a5af.agentmux\
  instances\
    default\
      wave-endpoints.json
        {
          "version": "0.27.11",
          "ws_endpoint": "127.0.0.1:PORT1",
          "web_endpoint": "127.0.0.1:PORT2",
          "auth_key": "...",
          "instance_id": ""
        }
    instance-1\
      wave-endpoints.json
        {
          "version": "0.27.11",
          "ws_endpoint": "127.0.0.1:PORT3",
          "web_endpoint": "127.0.0.1:PORT4",
          "auth_key": "...",
          "instance_id": "instance-1"
        }
    instance-2\
      wave-endpoints.json
        { ... }
```

**Benefits:**
- ✅ All data contained in single `com.a5af.agentmux` folder
- ✅ No `com.a5af.agentmux-instance-1` pollution
- ✅ Clear instance isolation
- ✅ Version tracking in endpoints file
- ✅ Easy to list all instances: `ls instances/*/wave-endpoints.json`
- ✅ Easy to clean up old versions

### Code Flow

**Backend (pkg/wavebase/wavebase.go:226-259)**
```go
func AcquireWaveLockWithAutoInstance() (FDLock, string, string, error) {
    // Try default instance first
    lock, err := AcquireWaveLock()
    if err == nil {
        return lock, "default", GetWaveDataDirForInstance("default"), nil
    }

    // Try instance-1 through instance-10
    for i := 1; i <= 10; i++ {
        instanceID := fmt.Sprintf("instance-%d", i)
        instanceDataDir := GetWaveDataDirForInstance(instanceID)
        // Returns: "%LOCALAPPDATA%\com.a5af.agentmux\instances\instance-1", etc.

        lock, err := acquireWaveLockAtPath(lockFileName)
        if err == nil {
            return lock, instanceID, instanceDataDir, nil
        }
    }
}

// GetWaveDataDirForInstance returns nested instance directory
func GetWaveDataDirForInstance(instanceID string) string {
    baseDir := GetWaveDataDir()  // %LOCALAPPDATA%\com.a5af.agentmux
    return filepath.Join(baseDir, "instances", instanceID)
}

// Example paths:
// - default:    %LOCALAPPDATA%\com.a5af.agentmux\instances\default
// - instance-1: %LOCALAPPDATA%\com.a5af.agentmux\instances\instance-1
// - instance-2: %LOCALAPPDATA%\com.a5af.agentmux\instances\instance-2
```

**Backend updates data directory (cmd/server/main-server.go:477-483)**
```go
if instanceID != "" && instanceDataDir != "" {
    // Update global cache to instance-specific directory
    wavebase.DataHome_VarCache = instanceDataDir
    log.Printf("[multi-instance] Running as instance: %s (data: %s)\n", instanceID, instanceDataDir)
}
```

**Frontend passes SAME data dir to ALL backends (src-tauri/src/sidecar.rs:133-134)**
```rust
.env("WAVETERM_CONFIG_HOME", config_dir.to_string_lossy().to_string())
.env("WAVETERM_DATA_HOME", data_dir.to_string_lossy().to_string())
// ❌ config_dir and data_dir are SAME for all instances!
```

**Frontend saves to SHARED file (src-tauri/src/sidecar.rs:224)**
```rust
let endpoints_file = config_dir.join("wave-endpoints.json");
// ❌ All instances write to %APPDATA%\com.a5af.agentmux\wave-endpoints.json
std::fs::write(&endpoints_file, &json)?;
```

---

## Solution Design

### Option 1: Instance-Aware Config Directory (Recommended)

**Approach:** Each frontend instance uses a separate config directory based on the backend's instance ID.

**Changes required:**

1. **Backend communicates instance ID to frontend**
   - Add instance ID to `WAVESRV-ESTART` message
   - Format: `WAVESRV-ESTART ws:... web:... version:... buildtime:... instance:<id>`

2. **Frontend reads instance ID and uses instance-specific config**
   - Parse instance ID from backend startup message
   - Save endpoints to instance-specific file:
     - Default: `%APPDATA%\com.a5af.agentmux\wave-endpoints.json`
     - Instance 1: `%APPDATA%\com.a5af.agentmux-instance-1\wave-endpoints.json`
     - Instance 2: `%APPDATA%\com.a5af.agentmux-instance-2\wave-endpoints.json`

**Pros:**
- ✅ Minimal changes
- ✅ Preserves backend multi-instance logic
- ✅ Each instance isolated
- ✅ No frontend changes before backend starts

**Cons:**
- ❌ Requires coordination between backend and frontend
- ❌ Must update endpoints file location after backend starts

### Option 2: Pre-allocated Instance via Command-Line Arg

**Approach:** Frontend assigns instance number BEFORE spawning backend.

**Changes required:**

1. **Frontend generates instance ID on startup**
   - Check for existing instances via lock files
   - Assign next available instance-N

2. **Pass instance ID to backend**
   - Add `--instance instance-1` command-line arg
   - Backend uses this instead of auto-detection

3. **Frontend uses instance-specific directories immediately**
   - Set `WAVETERM_DATA_HOME` to instance-specific path
   - Set `WAVETERM_CONFIG_HOME` to instance-specific path

**Pros:**
- ✅ Frontend knows instance ID from the start
- ✅ No endpoint file coordination needed
- ✅ Cleaner separation of concerns

**Cons:**
- ❌ More changes to backend
- ❌ Duplicates instance detection logic in frontend

### Option 3: Unique Endpoints File Per Process

**Approach:** Use process ID or random UUID in endpoints filename.

**Changes required:**

1. **Backend generates unique ID**
   - Add UUID or use process ID
   - Include in `WAVESRV-ESTART` message

2. **Frontend saves to unique file**
   - `wave-endpoints-<uuid>.json` or `wave-endpoints-<pid>.json`
   - No collisions possible

**Pros:**
- ✅ Zero collision risk
- ✅ Minimal logic changes

**Cons:**
- ❌ No way to reuse existing backend (breaks multi-window support)
- ❌ Orphaned endpoint files accumulate
- ❌ Must add cleanup logic

---

## Recommended Solution: Option 1

**Rationale:**
- Preserves existing backend multi-instance architecture
- Maintains backend reuse capability for multi-window support
- Minimal code changes
- Instance IDs are semantic ("instance-1", not "pid-12345")

### Implementation Plan

#### Phase 1: Backend Changes

**File:** `pkg/wavebase/wavebase.go`

```go
// Update GetWaveDataDirForInstance to use nested structure
func GetWaveDataDirForInstance(instanceID string) string {
    baseDir := DataHome_VarCache  // Already set from env vars
    if instanceID == "" {
        instanceID = "default"
    }
    return filepath.Join(baseDir, "instances", instanceID)
}

// Update AcquireWaveLockWithAutoInstance to return "default" for default instance
func AcquireWaveLockWithAutoInstance() (FDLock, string, string, error) {
    lock, err := AcquireWaveLock()
    if err == nil {
        // Return "default" instead of empty string for consistency
        return lock, "default", GetWaveDataDirForInstance("default"), nil
    }

    // Try instance-1 through instance-10
    for i := 1; i <= 10; i++ {
        instanceID := fmt.Sprintf("instance-%d", i)
        instanceDataDir := GetWaveDataDirForInstance(instanceID)
        lockFileName := filepath.Join(instanceDataDir, GetWaveLockFile())

        err := TryMkdirs(instanceDataDir, 0700, "instance data directory")
        if err != nil {
            continue
        }

        lock, err := acquireWaveLockAtPath(lockFileName)
        if err == nil {
            return lock, instanceID, instanceDataDir, nil
        }
    }
    return nil, "", "", fmt.Errorf("all 10 instances in use")
}
```

**File:** `cmd/server/main-server.go`

```go
// Line ~474: Update instance ID handling
CurrentInstanceID = instanceID  // Will be "default", "instance-1", etc.

// Line ~477: Always update data directory cache (even for default)
if instanceDataDir != "" {
    wavebase.DataHome_VarCache = instanceDataDir
    log.Printf("[multi-instance] Running as instance: %s (data: %s)\n", instanceID, instanceDataDir)
}

// Line ~613: Update WAVESRV-ESTART message format
fmt.Fprintf(os.Stderr, "WAVESRV-ESTART ws:%s web:%s version:%s buildtime:%s instance:%s\n",
    wsListener.Addr(),
    webListener.Addr(),
    WaveVersion,      // Already version-specific (backend tracks this)
    BuildTime,
    CurrentInstanceID, // "default", "instance-1", etc.
)
```

**Note:** Backend already has version isolation via version-specific lock/socket names:
- v0.27.11 uses: `wave-0.27.11.lock`, `wave-0.27.11.sock`
- v0.27.12 uses: `wave-0.27.12.lock`, `wave-0.27.12.sock`

Frontend just needs to track which version in endpoints file.

#### Phase 2: Frontend Changes

**File:** `src-tauri/src/sidecar.rs`

**1. Parse instance ID from backend message (lines 160-174)**

```rust
if l.starts_with("WAVESRV-ESTART") {
    let parts: Vec<&str> = l.split_whitespace().collect();
    let ws = parts.iter().find_map(|p| p.strip_prefix("ws:")).map(|s| s.to_string()).unwrap_or_default();
    let web = parts.iter().find_map(|p| p.strip_prefix("web:")).map(|s| s.to_string()).unwrap_or_default();
    let instance_id = parts.iter().find_map(|p| p.strip_prefix("instance:")).map(|s| s.to_string()).unwrap_or_default();

    tracing::info!("Backend started: ws={}, web={}, instance={}", ws, web, instance_id);
    let _ = tx.send((ws, web, instance_id)).await;
}
```

**2. Use instance-specific config directory (lines 224-250)**

```rust
// Compute nested instance directory inside base config dir
let instance_id = if result.instance_id.is_empty() {
    "default".to_string()
} else {
    result.instance_id.clone()
};

// Nested structure: %APPDATA%\com.a5af.agentmux\instances\{instance_id}\
let endpoints_dir = config_dir.join("instances").join(&instance_id);

// Ensure instance directory exists
std::fs::create_dir_all(&endpoints_dir)
    .map_err(|e| format!("Failed to create instance config dir: {}", e))?;

let endpoints_file = endpoints_dir.join("wave-endpoints.json");
tracing::info!("Saving endpoints to: {}", endpoints_file.display());

// Save endpoints with version tracking
let endpoints_json = serde_json::json!({
    "version": result.version,          // Track backend version
    "ws_endpoint": result.ws_endpoint,
    "web_endpoint": result.web_endpoint,
    "auth_key": result.auth_key,
    "instance_id": result.instance_id,
    "pid": std::process::id(),          // Track backend PID
    "started_at": chrono::Utc::now().to_rfc3339(),
});

match serde_json::to_string_pretty(&endpoints_json) {
    Ok(json) => {
        match std::fs::write(&endpoints_file, &json) {
            Ok(_) => {
                tracing::info!("✅ Saved endpoints to {}", endpoints_file.display());
            }
            Err(e) => {
                tracing::error!("❌ Failed to write endpoints: {}", e);
            }
        }
    }
    Err(e) => {
        tracing::error!("Failed to serialize endpoints: {}", e);
    }
}
```

**3. Check for existing backend in nested instance directories (lines 38-90)**

```rust
// Scan for any existing backend (checking all instances)
let instances_to_check = vec!["default", "instance-1", "instance-2", "instance-3",
                               "instance-4", "instance-5", "instance-6", "instance-7",
                               "instance-8", "instance-9", "instance-10"];

for instance_id in instances_to_check {
    let instance_dir = get_instance_config_dir(&config_dir, instance_id);
    let endpoints_file = instance_dir.join("wave-endpoints.json");

    if let Ok(existing) = try_load_endpoints(&endpoints_file, app).await {
        tracing::info!(
            "✅ Reusing existing backend v{} (instance: {})",
            existing.version,
            existing.instance_id
        );

        // Update auth key in app state
        let state = app.state::<crate::state::AppState>();
        let mut auth_key_guard = state.auth_key.lock().unwrap();
        *auth_key_guard = existing.auth_key.clone();

        return Ok(existing);
    }
}

tracing::info!("No existing backend found, spawning new one");
// Continue with spawn logic...
```

**Example file structure after running 3 instances:**

```
%APPDATA%\com.a5af.agentmux\
  instances\
    default\
      wave-endpoints.json
        {
          "version": "0.27.12",
          "instance_id": "default",
          "ws_endpoint": "127.0.0.1:58231",
          "web_endpoint": "127.0.0.1:58232",
          "auth_key": "abc123...",
          "pid": 12345,
          "started_at": "2026-02-15T12:00:00Z"
        }
    instance-1\
      wave-endpoints.json
        {
          "version": "0.27.12",
          "instance_id": "instance-1",
          "ws_endpoint": "127.0.0.1:58233",
          "web_endpoint": "127.0.0.1:58234",
          "auth_key": "def456...",
          "pid": 12346,
          "started_at": "2026-02-15T12:01:00Z"
        }
    instance-2\
      wave-endpoints.json
        { ... }

%LOCALAPPDATA%\com.a5af.agentmux\
  instances\
    default\
      wave-0.27.12.lock
      wave-0.27.12.sock
      db\
    instance-1\
      wave-0.27.12.lock
      wave-0.27.12.sock
      db\
    instance-2\
      wave-0.27.12.lock
      wave-0.27.12.sock
      db\
```

**4. Add helper functions**

```rust
/// Get instance-specific config directory (nested inside base config)
fn get_instance_config_dir(config_dir: &Path, instance_id: &str) -> PathBuf {
    config_dir.join("instances").join(instance_id)
}

/// Try to load and test endpoints from a file
async fn try_load_endpoints(
    endpoints_file: &Path,
    app: &tauri::AppHandle,
) -> Result<BackendSpawnResult, String> {
    if !endpoints_file.exists() {
        return Err("File not found".to_string());
    }

    let contents = std::fs::read_to_string(endpoints_file)
        .map_err(|e| format!("Read error: {}", e))?;

    // Parse endpoints JSON (with version field)
    let json: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Parse error: {}", e))?;

    let existing = BackendSpawnResult {
        ws_endpoint: json["ws_endpoint"].as_str().unwrap_or_default().to_string(),
        web_endpoint: json["web_endpoint"].as_str().unwrap_or_default().to_string(),
        auth_key: json["auth_key"].as_str().unwrap_or_default().to_string(),
        instance_id: json["instance_id"].as_str().unwrap_or_default().to_string(),
        version: json["version"].as_str().unwrap_or_default().to_string(),
    };

    // Verify version matches current build
    let current_version = env!("CARGO_PKG_VERSION");
    if existing.version != current_version {
        tracing::warn!(
            "Backend version mismatch: file={}, current={}",
            existing.version,
            current_version
        );
        // Don't reuse backend if version differs
        return Err(format!("Version mismatch: {} vs {}", existing.version, current_version));
    }

    // Test if backend is responsive
    let test_url = if existing.web_endpoint.starts_with("http") {
        existing.web_endpoint.clone()
    } else {
        format!("http://{}", existing.web_endpoint)
    };

    match reqwest::get(&test_url).await {
        Ok(resp) if resp.status().is_success() || resp.status().is_client_error() => {
            tracing::info!("Found responsive backend v{} at: {}", existing.version, test_url);
            Ok(existing)
        }
        _ => {
            tracing::warn!("Backend not responsive, removing stale file");
            let _ = std::fs::remove_file(endpoints_file);
            Err("Backend not responsive".to_string())
        }
    }
}

/// Scan for all running instances (any version)
fn list_all_instances(config_dir: &Path) -> Vec<PathBuf> {
    let instances_dir = config_dir.join("instances");
    if !instances_dir.exists() {
        return vec![];
    }

    let mut endpoints_files = vec![];
    if let Ok(entries) = std::fs::read_dir(&instances_dir) {
        for entry in entries.flatten() {
            let endpoint_file = entry.path().join("wave-endpoints.json");
            if endpoint_file.exists() {
                endpoints_files.push(endpoint_file);
            }
        }
    }
    endpoints_files
}
```

#### Phase 3: Update BackendSpawnResult

**File:** `src-tauri/src/sidecar.rs`

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackendSpawnResult {
    pub ws_endpoint: String,
    pub web_endpoint: String,
    pub auth_key: String,
    pub instance_id: String,  // "default", "instance-1", etc.
    pub version: String,      // Backend version (e.g., "0.27.12")
}
```

**Parse version from WAVESRV-ESTART (lines 160-174):**

```rust
if l.starts_with("WAVESRV-ESTART") {
    let parts: Vec<&str> = l.split_whitespace().collect();
    let ws = parts.iter().find_map(|p| p.strip_prefix("ws:")).map(|s| s.to_string()).unwrap_or_default();
    let web = parts.iter().find_map(|p| p.strip_prefix("web:")).map(|s| s.to_string()).unwrap_or_default();
    let version = parts.iter().find_map(|p| p.strip_prefix("version:")).map(|s| s.to_string()).unwrap_or_default();
    let instance_id = parts.iter().find_map(|p| p.strip_prefix("instance:")).map(|s| s.to_string()).unwrap_or_default();

    tracing::info!("Backend started: ws={}, web={}, version={}, instance={}", ws, web, version, instance_id);
    let _ = tx.send((ws, web, version, instance_id)).await;
}
```

**Create result with version (lines 214-218):**

```rust
let result = BackendSpawnResult {
    ws_endpoint: timeout.0,
    web_endpoint: timeout.1,
    version: timeout.2,           // From WAVESRV-ESTART
    instance_id: timeout.3,       // From WAVESRV-ESTART
    auth_key: auth_key.clone(),
};
```

### Testing Plan

1. **Single instance test**
   - Start instance 1
   - Verify endpoints saved to `%APPDATA%\com.a5af.agentmux\wave-endpoints.json`
   - Verify window title shows "AgentMux" (no instance number)

2. **Two instance test**
   - Start instance 1
   - Start instance 2
   - Verify instance 1 still accepts input
   - Verify instance 2 endpoints saved to `%APPDATA%\com.a5af.agentmux-instance-1\wave-endpoints.json`
   - Verify instance 2 window title shows "AgentMux (instance-1)"

3. **Three instance test**
   - Start instances 1, 2, 3
   - Type in instance 1 → appears in instance 1 ✅
   - Type in instance 2 → appears in instance 2 ✅
   - Type in instance 3 → appears in instance 3 ✅

4. **Instance restart test**
   - Start instance 1
   - Close instance 1 window (backend stays running)
   - Reopen AgentMux
   - Verify reconnects to same backend (reuses endpoints file)

5. **Multi-window test**
   - Start instance 1
   - Open second window (Ctrl+N or File → New Window)
   - Verify second window reuses instance 1 backend
   - Both windows share same terminals ✅

---

## Migration Notes

### Backwards Compatibility

**Existing users (v0.27.11):**
- Old endpoint file at `%APPDATA%\com.a5af.agentmux\wave-endpoints.json` will be ignored
- Frontend will check new location first: `instances\default\wave-endpoints.json`
- Migration happens automatically on first run of v0.27.12

**Migration strategy:**

```rust
// On startup, check both old and new locations
let old_endpoints = config_dir.join("wave-endpoints.json");
let new_endpoints = config_dir.join("instances").join("default").join("wave-endpoints.json");

if !new_endpoints.exists() && old_endpoints.exists() {
    // Migrate old file to new location
    if let Ok(contents) = std::fs::read_to_string(&old_endpoints) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&contents) {
            // Add missing fields
            json["version"] = serde_json::json!(env!("CARGO_PKG_VERSION"));
            json["instance_id"] = serde_json::json!("default");

            std::fs::create_dir_all(new_endpoints.parent().unwrap())?;
            std::fs::write(&new_endpoints, serde_json::to_string_pretty(&json)?)?;

            tracing::info!("✅ Migrated old endpoints file to new location");

            // Remove old file
            let _ = std::fs::remove_file(&old_endpoints);
        }
    }
}
```

**Config file locations:**
```
Before fix (v0.27.11):
  %APPDATA%\com.a5af.agentmux\
    wave-endpoints.json  ❌ ALL instances shared this file

After fix (v0.27.12):
  %APPDATA%\com.a5af.agentmux\
    instances\
      default\
        wave-endpoints.json    ✅ Default instance
      instance-1\
        wave-endpoints.json    ✅ Second instance
      instance-2\
        wave-endpoints.json    ✅ Third instance
```

**Data directory locations:**
```
Before fix (v0.27.11):
  %LOCALAPPDATA%\com.a5af.agentmux\              ← Default
  %LOCALAPPDATA%\com.a5af.agentmux-instance-1\  ← Instance 1
  %LOCALAPPDATA%\com.a5af.agentmux-instance-2\  ← Instance 2

After fix (v0.27.12):
  %LOCALAPPDATA%\com.a5af.agentmux\
    instances\
      default\      ← Default
      instance-1\   ← Instance 1
      instance-2\   ← Instance 2
```

**Benefits of nested structure:**
- ✅ All AgentMux data in one folder
- ✅ Easy cleanup: delete `com.a5af.agentmux` folder
- ✅ Easy backup: copy `com.a5af.agentmux` folder
- ✅ Version isolation preserved (different versions use different socket names)
- ✅ No `com.a5af.agentmux-instance-1` folders polluting AppData

---

## Version Detection and Multi-Version Support

**Backend already isolates versions:**

The backend uses version-specific lock and socket files:
```go
func GetWaveLockFile() string {
    return fmt.Sprintf("wave-%s.lock", WaveVersion)  // e.g., "wave-0.27.12.lock"
}

func GetDomainSocketBaseName() string {
    return fmt.Sprintf("wave-%s.sock", WaveVersion)  // e.g., "wave-0.27.12.sock"
}
```

**This means:**
- v0.27.11 backend: `wave-0.27.11.lock`, `wave-0.27.11.sock`
- v0.27.12 backend: `wave-0.27.12.lock`, `wave-0.27.12.sock`
- Different versions can run simultaneously without conflict ✅

**Frontend version checking:**

When reusing an existing backend, verify version matches:

```rust
async fn try_load_endpoints(endpoints_file: &Path, app: &tauri::AppHandle) -> Result<BackendSpawnResult, String> {
    // ... load endpoints file ...

    let existing_version = json["version"].as_str().unwrap_or_default();
    let current_version = env!("CARGO_PKG_VERSION");

    if existing_version != current_version {
        tracing::warn!("Version mismatch: backend={}, frontend={}", existing_version, current_version);
        return Err(format!("Version mismatch"));
    }

    // Version matches - safe to reuse backend
    Ok(existing)
}
```

**Scenario: Running v0.27.11 and v0.27.12 simultaneously**

```
%LOCALAPPDATA%\com.a5af.agentmux\
  instances\
    default\
      wave-0.27.11.lock     ← v0.27.11 backend (instance 1)
      wave-0.27.11.sock
    instance-1\
      wave-0.27.12.lock     ← v0.27.12 backend (instance 1)
      wave-0.27.12.sock
    instance-2\
      wave-0.27.11.lock     ← v0.27.11 backend (instance 2)
      wave-0.27.11.sock

%APPDATA%\com.a5af.agentmux\
  instances\
    default\
      wave-endpoints.json
        { "version": "0.27.11", "instance_id": "default", ... }
    instance-1\
      wave-endpoints.json
        { "version": "0.27.12", "instance_id": "instance-1", ... }
    instance-2\
      wave-endpoints.json
        { "version": "0.27.11", "instance_id": "instance-2", ... }
```

**Version isolation ensures:**
- v0.27.11 frontend only connects to v0.27.11 backend
- v0.27.12 frontend only connects to v0.27.12 backend
- No cross-version contamination ✅

---

## Version Bump

This fix requires a version bump to **v0.27.12** because:
- Changes backend message format (`WAVESRV-ESTART` now includes `instance:<id>`)
- Changes frontend config file locations
- Fixes critical P0 bug affecting multi-instance users

---

## References

- **Backend multi-instance code:** `pkg/wavebase/wavebase.go:214-259`
- **Backend startup:** `cmd/server/main-server.go:433-616`
- **Frontend sidecar spawn:** `src-tauri/src/sidecar.rs:22-253`
- **Issue report:** User message (2026-02-15)

---

## Appendix: Debugging Commands

### Check running instances

**Windows PowerShell:**
```powershell
# List all agentmuxsrv processes
Get-Process agentmuxsrv | Format-Table Id, StartTime, Path

# Check lock files (nested structure)
Get-ChildItem "$env:LOCALAPPDATA\com.a5af.agentmux\instances\*\wave-*.lock"

# Check endpoint files (nested structure)
Get-ChildItem "$env:APPDATA\com.a5af.agentmux\instances\*\wave-endpoints.json"

# List all instances with version info
Get-ChildItem "$env:APPDATA\com.a5af.agentmux\instances\*\wave-endpoints.json" | ForEach-Object {
    $json = Get-Content $_.FullName | ConvertFrom-Json
    [PSCustomObject]@{
        Instance = $_.Directory.Name
        Version = $json.version
        WsEndpoint = $json.ws_endpoint
        WebEndpoint = $json.web_endpoint
        PID = $json.pid
        StartedAt = $json.started_at
    }
} | Format-Table -AutoSize

# Clean up stale instances (backends that died)
Get-ChildItem "$env:APPDATA\com.a5af.agentmux\instances\*\wave-endpoints.json" | ForEach-Object {
    $json = Get-Content $_.FullName | ConvertFrom-Json
    $pid = $json.pid
    if ($pid -and -not (Get-Process -Id $pid -ErrorAction SilentlyContinue)) {
        Write-Host "Removing stale instance: $($_.Directory.Name) (pid $pid not running)"
        Remove-Item $_.Directory.FullName -Recurse -Force
    }
}
```

**Bash (Linux/macOS):**
```bash
# List all agentmuxsrv processes
ps aux | grep agentmuxsrv

# Check lock files
find ~/.local/share/com.a5af.agentmux/instances -name "wave-*.lock"

# Check endpoint files
find ~/.config/com.a5af.agentmux/instances -name "wave-endpoints.json"

# List all instances with version info
for f in ~/.config/com.a5af.agentmux/instances/*/wave-endpoints.json; do
    echo "=== $(basename $(dirname $f)) ==="
    jq '{version, instance_id, pid, started_at}' "$f"
done

# Clean up stale instances
for f in ~/.config/com.a5af.agentmux/instances/*/wave-endpoints.json; do
    pid=$(jq -r '.pid' "$f")
    if ! kill -0 "$pid" 2>/dev/null; then
        instance=$(basename $(dirname $f))
        echo "Removing stale instance: $instance (pid $pid not running)"
        rm -rf "$(dirname $f)"
    fi
done
```

### Reproduce the bug

1. Extract portable build
2. Launch `agentmux.exe` (instance 1)
3. Launch `agentmux.exe` again (instance 2)
4. Type in instance 1 terminal → **INPUT DOESN'T APPEAR**
5. Type in instance 2 terminal → **INPUT APPEARS**
6. Open new tab in instance 1 → **BLANK PANE**

### Verify the fix

1. Build with fix applied
2. Launch instance 1 → check `%APPDATA%\com.a5af.agentmux\instances\default\wave-endpoints.json` created
3. Launch instance 2 → check `%APPDATA%\com.a5af.agentmux\instances\instance-1\wave-endpoints.json` created
4. Launch instance 3 → check `%APPDATA%\com.a5af.agentmux\instances\instance-2\wave-endpoints.json` created
5. Type in instance 1 → **INPUT APPEARS IN INSTANCE 1** ✅
6. Type in instance 2 → **INPUT APPEARS IN INSTANCE 2** ✅
7. Type in instance 3 → **INPUT APPEARS IN INSTANCE 3** ✅
8. Open new tab in instance 1 → **TAB OPENS IN INSTANCE 1** ✅
9. Close instance 2, reopen → **RECONNECTS TO INSTANCE 2 BACKEND** ✅

**Check file structure:**
```powershell
tree /F "$env:APPDATA\com.a5af.agentmux\instances"

# Expected output:
# instances
# ├── default
# │   └── wave-endpoints.json
# ├── instance-1
# │   └── wave-endpoints.json
# └── instance-2
#     └── wave-endpoints.json
```

**Verify version tracking:**
```powershell
Get-Content "$env:APPDATA\com.a5af.agentmux\instances\default\wave-endpoints.json" | ConvertFrom-Json | Select version, instance_id

# Expected output:
# version    instance_id
# -------    -----------
# 0.27.12    default
```

---

**End of Specification**
