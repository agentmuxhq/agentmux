# WaveTerm Comprehensive Crash Reporting Specification

**Version:** 1.0
**Date:** 2025-10-19
**Status:** Design Phase
**Target:** WaveTerm Fork v0.12.4+
**Supersedes:** SPEC_CRASH_HANDLER_MODAL.md (integrates and extends)

---

## Executive Summary

Implement a **multi-layer crash detection and reporting system** that captures ALL types of crashes:
1. JavaScript exceptions (uncaught errors)
2. Native crashes (GPU, renderer, V8)
3. Process terminations (Task Manager, SIGKILL)
4. Out-of-memory kills

The system uses a combination of:
- **Real-time modals** for immediate crash notification
- **Native crash dumps** for post-mortem analysis
- **Heartbeat monitoring** for detecting silent kills
- **Recovery modal** on next launch after crash

---

## 1. Current State

### 1.1 What WaveTerm Has Now

**Partial JavaScript Exception Handling:**
- `emain/emain.ts:655` - `process.on('uncaughtException')`
- Logs to `waveapp.log` and quits
- No user notification
- No crash dumps

**Missing:**
- ❌ No `crashReporter` configured (no native crash dumps)
- ❌ No GPU/renderer crash handlers
- ❌ No process monitoring/heartbeat
- ❌ No recovery modal on restart
- ❌ No breadcrumb logging

### 1.2 Evidence from Recent Crash

**Incident:** Oct 19, 2025 @ 11:38 AM

**Findings:**
- Process terminated mid-operation (filestore flush)
- No shutdown sequence in logs
- No JavaScript exception
- No crash dumps found
- Lock file empty (0 bytes)
- Database WAL files show activity stopped abruptly

**Conclusion:** Likely **native crash** (GPU/renderer) that would have been caught by crashReporter.

---

## 2. Crash Types and Detection Strategy

### 2.1 Coverage Matrix

| Crash Type | Real-Time Modal | crashReporter | Heartbeat | Next Launch Recovery |
|------------|----------------|---------------|-----------|---------------------|
| **JS Exception** | ✅ Modal + log | ❌ | ❌ | ⚠️ Check log |
| **Native Crash** (GPU/V8) | ⚠️ Event handler | ✅ Minidump | ❌ | ✅ Check Crashpad |
| **Process Kill** (external) | ❌ | ❌ | ✅ Stale heartbeat | ✅ Detect stale |
| **OOM Kill** | ⚠️ Maybe event | ⚠️ Maybe dump | ✅ Stale heartbeat | ✅ Detect stale |

### 2.2 Detection Layers

**Layer 1: Real-Time Detection** (before exit)
```typescript
// JavaScript exceptions
process.on('uncaughtException')
process.on('unhandledRejection')

// GPU crashes
app.on('gpu-process-crashed')

// Renderer crashes
webContents.on('render-process-gone')

// OOM warnings
app.on('renderer-process-crashed') // includes OOM
```

**Layer 2: Native Crash Dumps** (process crashes)
```typescript
crashReporter.start({...})
// Saves minidumps to Crashpad/reports/
```

**Layer 3: Heartbeat Monitor** (silent kills)
```typescript
// Write timestamp every 5 seconds
setInterval(() => {
  fs.writeFileSync(heartbeatFile, Date.now())
}, 5000)

// On next launch, check if heartbeat is stale
```

**Layer 4: Recovery on Restart** (next launch)
```typescript
// Check for:
// 1. Crash dumps in Crashpad/
// 2. Stale heartbeat file
// 3. Unclean shutdown flag
// → Show recovery modal
```

---

## 3. Implementation Plan

### 3.1 Phase 1: Native Crash Reporter

**File:** `emain/crash-reporter.ts`

```typescript
import * as electron from "electron";
import * as path from "path";
import { getWaveDataDir } from "./platform";

export function initCrashReporter() {
    const waveDataDir = getWaveDataDir();
    const crashesDir = path.join(waveDataDir, "crashes");

    electron.crashReporter.start({
        productName: "WaveTerm",
        companyName: "CommandLine",
        submitURL: "", // Local-only for now
        uploadToServer: false, // Don't send to remote server
        compress: true,
        ignoreSystemCrashHandler: false,
        rateLimit: false,
        globalExtra: {
            // Additional metadata for all crashes
            "waveVersion": require("../package.json").version,
            "platform": process.platform,
            "arch": process.arch,
        },
    });

    console.log("crashReporter initialized, dumps will be saved to:", crashesDir);
    console.log("Crashpad database:", electron.crashReporter.getCrashesDirectory());
}
```

**Crashpad Directory Structure:**
```
C:\Users\asafe\AppData\Local\waveterm-auto-{id}\Crashpad\
├── db/                      # Crashpad database
├── reports/                 # Pending uploads (empty since uploadToServer=false)
├── completed/              # Processed dumps
└── settings.dat            # Crashpad config
```

**Call from `emain/emain.ts`:**
```typescript
import { initCrashReporter } from "./crash-reporter";

// Very early in appMain(), before app.whenReady()
initCrashReporter();
```

### 3.2 Phase 2: Real-Time Crash Handlers

**File:** `emain/crash-handlers.ts`

```typescript
import * as electron from "electron";
import { showCrashDialog } from "./crash-handler"; // From SPEC_CRASH_HANDLER_MODAL
import { writeCrashBreadcrumb, getCrashBreadcrumbs } from "./crash-breadcrumbs";

/**
 * Initialize all real-time crash event handlers
 */
export function initCrashHandlers() {
    // 1. JavaScript exceptions (already in emain.ts)
    process.on("uncaughtException", async (error) => {
        writeCrashBreadcrumb("uncaughtException", { error: error.message });
        await showCrashDialog(error);
        electron.app.quit();
    });

    process.on("unhandledRejection", async (reason) => {
        const error = reason instanceof Error ? reason : new Error(String(reason));
        writeCrashBreadcrumb("unhandledRejection", { reason: String(reason) });
        await showCrashDialog(error);
        electron.app.quit();
    });

    // 2. GPU process crashes
    electron.app.on("gpu-process-crashed", (event, killed) => {
        console.log("GPU process crashed, killed:", killed);
        writeCrashBreadcrumb("gpu-process-crashed", { killed });

        electron.dialog.showMessageBoxSync({
            type: "error",
            title: "GPU Process Crashed",
            message: "The graphics processor has crashed.",
            detail:
                "WaveTerm's GPU process has crashed. This may be due to:\n" +
                "• Outdated graphics drivers\n" +
                "• Hardware acceleration issues\n" +
                "• GPU memory exhaustion\n\n" +
                "You can disable hardware acceleration in settings.\n\n" +
                `Killed: ${killed}`,
            buttons: ["Restart", "Quit"],
        });

        electron.app.relaunch();
        electron.app.quit();
    });

    // 3. Renderer process crashes
    electron.app.on("render-process-gone", (event, webContents, details) => {
        console.log("Renderer process gone:", details.reason);
        writeCrashBreadcrumb("render-process-gone", {
            reason: details.reason,
            exitCode: details.exitCode
        });

        const reasonMessages: Record<string, string> = {
            "clean-exit": "Renderer exited cleanly (unexpected)",
            "abnormal-exit": "Renderer crashed",
            "killed": "Renderer was killed",
            "crashed": "Renderer crashed",
            "oom": "Out of memory",
            "launch-failed": "Failed to launch renderer",
            "integrity-failure": "Code integrity check failed",
        };

        const message = reasonMessages[details.reason] || `Unknown reason: ${details.reason}`;

        electron.dialog.showMessageBoxSync({
            type: "error",
            title: "Renderer Process Crashed",
            message: "The rendering process has crashed.",
            detail:
                `Reason: ${message}\n` +
                `Exit Code: ${details.exitCode}\n\n` +
                (details.reason === "oom" ?
                    "This may be due to:\n" +
                    "• Too many tabs/windows open\n" +
                    "• Memory leak in a block\n" +
                    "• Large file operations\n\n"
                    : "") +
                "WaveTerm will restart.",
            buttons: ["Restart"],
        });

        electron.app.relaunch();
        electron.app.quit();
    });

    // 4. Child process (wavesrv) crashes
    // Already handled in emain-wavesrv.ts but can add breadcrumb
}
```

### 3.3 Phase 3: Heartbeat Monitor

**File:** `emain/heartbeat.ts`

```typescript
import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir } from "./platform";

const HEARTBEAT_INTERVAL = 5000; // 5 seconds
const HEARTBEAT_STALE_MS = 30000; // 30 seconds = stale
let heartbeatTimer: NodeJS.Timeout | null = null;

export function getHeartbeatFilePath(): string {
    return path.join(getWaveDataDir(), "heartbeat.txt");
}

export function startHeartbeat() {
    const heartbeatFile = getHeartbeatFilePath();

    // Write initial heartbeat
    writeHeartbeat(heartbeatFile);

    // Update every 5 seconds
    heartbeatTimer = setInterval(() => {
        writeHeartbeat(heartbeatFile);
    }, HEARTBEAT_INTERVAL);

    console.log("Heartbeat monitor started:", heartbeatFile);
}

function writeHeartbeat(file: string) {
    try {
        const data = {
            timestamp: Date.now(),
            pid: process.pid,
            version: require("../package.json").version,
        };
        fs.writeFileSync(file, JSON.stringify(data, null, 2));
    } catch (e) {
        console.error("Failed to write heartbeat:", e);
    }
}

export function stopHeartbeat() {
    if (heartbeatTimer) {
        clearInterval(heartbeatTimer);
        heartbeatTimer = null;
    }

    // Write final heartbeat with clean exit flag
    const heartbeatFile = getHeartbeatFilePath();
    try {
        const data = {
            timestamp: Date.now(),
            pid: process.pid,
            cleanExit: true,
        };
        fs.writeFileSync(heartbeatFile, JSON.stringify(data, null, 2));
    } catch (e) {
        console.error("Failed to write final heartbeat:", e);
    }
}

export function checkForStaleCrash(): { crashed: boolean; reason: string; data?: any } {
    const heartbeatFile = getHeartbeatFilePath();

    if (!fs.existsSync(heartbeatFile)) {
        return { crashed: false, reason: "no-heartbeat-file" };
    }

    try {
        const content = fs.readFileSync(heartbeatFile, "utf-8");
        const data = JSON.parse(content);

        // Check if clean exit
        if (data.cleanExit) {
            return { crashed: false, reason: "clean-exit" };
        }

        // Check if stale
        const now = Date.now();
        const age = now - data.timestamp;

        if (age > HEARTBEAT_STALE_MS) {
            return {
                crashed: true,
                reason: "stale-heartbeat",
                data: {
                    lastHeartbeat: new Date(data.timestamp).toISOString(),
                    ageSeconds: Math.floor(age / 1000),
                    pid: data.pid,
                    version: data.version,
                }
            };
        }

        // Heartbeat exists and is fresh (previous instance still running?)
        return { crashed: false, reason: "fresh-heartbeat", data };

    } catch (e) {
        console.error("Failed to check heartbeat:", e);
        return { crashed: false, reason: "error-reading-heartbeat" };
    }
}
```

**Integration in `emain/emain.ts`:**
```typescript
import { startHeartbeat, stopHeartbeat } from "./heartbeat";

async function appMain() {
    // ... after app.whenReady() ...

    // Start heartbeat
    startHeartbeat();

    // Stop on clean quit
    electron.app.on("before-quit", () => {
        stopHeartbeat();
    });
}
```

### 3.4 Phase 4: Crash Breadcrumbs

**File:** `emain/crash-breadcrumbs.ts`

```typescript
import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir } from "./platform";

interface Breadcrumb {
    timestamp: number;
    type: string;
    data: any;
}

const MAX_BREADCRUMBS = 100;
const breadcrumbs: Breadcrumb[] = [];

export function writeCrashBreadcrumb(type: string, data: any) {
    breadcrumbs.push({
        timestamp: Date.now(),
        type,
        data,
    });

    // Keep only last N
    if (breadcrumbs.length > MAX_BREADCRUMBS) {
        breadcrumbs.shift();
    }

    // Also write to file immediately (in case we crash)
    saveBreadcrumbsToFile();
}

export function getCrashBreadcrumbs(): Breadcrumb[] {
    return breadcrumbs;
}

function getBreadcrumbsFilePath(): string {
    return path.join(getWaveDataDir(), "crash-breadcrumbs.json");
}

function saveBreadcrumbsToFile() {
    try {
        fs.writeFileSync(
            getBreadcrumbsFilePath(),
            JSON.stringify(breadcrumbs, null, 2)
        );
    } catch (e) {
        // Ignore errors (don't want breadcrumb writing to cause crashes)
    }
}

export function loadBreadcrumbsFromFile(): Breadcrumb[] {
    try {
        const file = getBreadcrumbsFilePath();
        if (fs.existsSync(file)) {
            const content = fs.readFileSync(file, "utf-8");
            return JSON.parse(content);
        }
    } catch (e) {
        console.error("Failed to load breadcrumbs:", e);
    }
    return [];
}

// Track important events
export function trackEvent(type: string, data?: any) {
    writeCrashBreadcrumb(type, data);
}
```

**Usage Examples:**
```typescript
// Track window events
electron.ipcMain.on("window-created", (event, windowId) => {
    trackEvent("window-created", { windowId });
});

// Track tab events
electron.ipcMain.on("tab-switched", (event, tabId) => {
    trackEvent("tab-switched", { tabId });
});

// Track command execution
electron.ipcMain.on("command-executed", (event, command) => {
    trackEvent("command-executed", { command });
});
```

### 3.5 Phase 5: Recovery Modal

**File:** `emain/crash-recovery.ts`

```typescript
import * as electron from "electron";
import * as fs from "fs";
import * as path from "path";
import { checkForStaleCrash } from "./heartbeat";
import { loadBreadcrumbsFromFile } from "./crash-breadcrumbs";
import { getWaveDataDir } from "./platform";

export interface CrashInfo {
    type: "stale-heartbeat" | "crash-dump" | "unclean-shutdown";
    timestamp?: string;
    crashDumps?: string[];
    breadcrumbs?: any[];
    heartbeatData?: any;
}

export function checkForPreviousCrash(): CrashInfo | null {
    const results: CrashInfo = {
        type: "unclean-shutdown",
        crashDumps: [],
        breadcrumbs: [],
    };

    let foundCrash = false;

    // 1. Check heartbeat
    const heartbeatCheck = checkForStaleCrash();
    if (heartbeatCheck.crashed) {
        foundCrash = true;
        results.type = "stale-heartbeat";
        results.heartbeatData = heartbeatCheck.data;
        results.timestamp = heartbeatCheck.data?.lastHeartbeat;
    }

    // 2. Check for crash dumps
    const crashesDir = electron.crashReporter.getCrashesDirectory();
    if (fs.existsSync(crashesDir)) {
        const reports = path.join(crashesDir, "completed");
        if (fs.existsSync(reports)) {
            const dumps = fs.readdirSync(reports)
                .filter(f => f.endsWith(".dmp"))
                .map(f => path.join(reports, f));

            if (dumps.length > 0) {
                foundCrash = true;
                results.type = "crash-dump";
                results.crashDumps = dumps;

                // Get newest dump's timestamp
                const stats = fs.statSync(dumps[0]);
                results.timestamp = stats.mtime.toISOString();
            }
        }
    }

    // 3. Load breadcrumbs
    results.breadcrumbs = loadBreadcrumbsFromFile();

    return foundCrash ? results : null;
}

export async function showRecoveryModal(crashInfo: CrashInfo): Promise<void> {
    await electron.app.whenReady();

    const { dialog, shell, clipboard } = electron;

    const crashTypeMessages: Record<string, string> = {
        "stale-heartbeat": "Process was terminated unexpectedly",
        "crash-dump": "Application crashed (native crash detected)",
        "unclean-shutdown": "Application did not shut down cleanly",
    };

    const message = crashTypeMessages[crashInfo.type] || "Unknown crash type";

    const details = formatCrashDetails(crashInfo);

    const dialogOpts: Electron.MessageBoxOptions = {
        type: "warning",
        buttons: ["Copy Crash Info", "Clear Crashes", "Continue"],
        defaultId: 2, // Continue
        cancelId: 2,
        title: "WaveTerm Recovered from Crash",
        message: `WaveTerm recovered from a previous crash.\n\n${message}`,
        detail:
            `Last activity: ${crashInfo.timestamp || "Unknown"}\n\n` +
            (crashInfo.crashDumps && crashInfo.crashDumps.length > 0
                ? `Crash dumps found: ${crashInfo.crashDumps.length}\n`
                : "") +
            (crashInfo.breadcrumbs && crashInfo.breadcrumbs.length > 0
                ? `Recent actions: ${crashInfo.breadcrumbs.length} events\n`
                : "") +
            `\nClick "Copy Crash Info" to copy details for bug report.`,
        noLink: true,
    };

    const choice = dialog.showMessageBoxSync(dialogOpts);

    switch (choice) {
        case 0: // Copy Crash Info
            clipboard.writeText(details);
            dialog.showMessageBoxSync({
                type: "info",
                buttons: ["OK"],
                title: "Copied",
                message: "Crash information copied to clipboard",
            });
            break;

        case 1: // Clear Crashes
            clearCrashData(crashInfo);
            dialog.showMessageBoxSync({
                type: "info",
                buttons: ["OK"],
                title: "Cleared",
                message: "Crash data has been cleared.",
            });
            break;

        case 2: // Continue
        default:
            // Just continue, leave crash data for later analysis
            break;
    }
}

function formatCrashDetails(crashInfo: CrashInfo): string {
    let details = `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
WAVETERM CRASH RECOVERY REPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Crash Type: ${crashInfo.type}
Timestamp: ${crashInfo.timestamp || "Unknown"}

`;

    if (crashInfo.heartbeatData) {
        details += `━━━ HEARTBEAT DATA ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Last Heartbeat: ${crashInfo.heartbeatData.lastHeartbeat}
Age: ${crashInfo.heartbeatData.ageSeconds}s
Process ID: ${crashInfo.heartbeatData.pid}
Version: ${crashInfo.heartbeatData.version}

`;
    }

    if (crashInfo.crashDumps && crashInfo.crashDumps.length > 0) {
        details += `━━━ CRASH DUMPS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Found ${crashInfo.crashDumps.length} crash dump(s):
${crashInfo.crashDumps.map(d => `  - ${d}`).join("\n")}

`;
    }

    if (crashInfo.breadcrumbs && crashInfo.breadcrumbs.length > 0) {
        details += `━━━ RECENT EVENTS (Last ${crashInfo.breadcrumbs.length}) ━━━━━━━━━━━━━━━━━━━━

${crashInfo.breadcrumbs
    .slice(-20) // Last 20
    .map(b => {
        const time = new Date(b.timestamp).toISOString();
        const data = JSON.stringify(b.data);
        return `[${time}] ${b.type}: ${data}`;
    })
    .join("\n")}

`;
    }

    details += `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`;

    return details;
}

function clearCrashData(crashInfo: CrashInfo) {
    // Clear crash dumps
    if (crashInfo.crashDumps) {
        for (const dump of crashInfo.crashDumps) {
            try {
                fs.unlinkSync(dump);
            } catch (e) {
                console.error("Failed to delete crash dump:", e);
            }
        }
    }

    // Clear breadcrumbs
    const breadcrumbsFile = path.join(getWaveDataDir(), "crash-breadcrumbs.json");
    if (fs.existsSync(breadcrumbsFile)) {
        try {
            fs.unlinkSync(breadcrumbsFile);
        } catch (e) {
            console.error("Failed to delete breadcrumbs:", e);
        }
    }
}
```

**Integration in `emain/emain.ts`:**
```typescript
import { checkForPreviousCrash, showRecoveryModal } from "./crash-recovery";

async function appMain() {
    // ... early in appMain(), after initCrashReporter() ...

    // Check for previous crash BEFORE starting app
    const crashInfo = checkForPreviousCrash();
    if (crashInfo) {
        console.log("Previous crash detected:", crashInfo.type);
        await showRecoveryModal(crashInfo);
    }

    // Continue with normal startup...
}
```

---

## 4. Complete Integration

### 4.1 Modified `emain/emain.ts`

```typescript
// At top of file
import { initCrashReporter } from "./crash-reporter";
import { initCrashHandlers } from "./crash-handlers";
import { startHeartbeat, stopHeartbeat } from "./heartbeat";
import { checkForPreviousCrash, showRecoveryModal } from "./crash-recovery";
import { trackEvent } from "./crash-breadcrumbs";

// Very first thing in file (before any other imports run)
initCrashReporter();

async function appMain() {
    // ... after initial settings ...

    // 1. Check for previous crash
    const crashInfo = checkForPreviousCrash();
    if (crashInfo) {
        await showRecoveryModal(crashInfo);
    }

    // 2. Initialize crash handlers
    initCrashHandlers();

    // ... wait for app.whenReady() ...

    // 3. Start heartbeat monitor
    startHeartbeat();

    // 4. Track startup event
    trackEvent("app-started", {
        version: getWaveVersion().version,
        platform: unamePlatform,
        arch: unameArch,
    });

    // ... rest of startup ...

    // 5. Stop heartbeat on clean quit
    electron.app.on("before-quit", () => {
        trackEvent("app-quitting");
        stopHeartbeat();
    });
}
```

### 4.2 Event Tracking Throughout App

**Example: Track window creation**
```typescript
// In emain-window.ts
export async function createBrowserWindow(...) {
    // ...
    trackEvent("window-created", {
        windowId: windowData.oid,
        workspaceId: windowData.workspaceid,
    });
    // ...
}
```

**Example: Track tab switches**
```typescript
// In emain-window.ts
async setActiveTab(tabId: string) {
    trackEvent("tab-switched", { tabId, windowId: this.waveWindowId });
    // ...
}
```

**Example: Track command execution**
```typescript
// In emain-wsh.ts
electron.ipcMain.handle("run-command", (event, command) => {
    trackEvent("command-executed", { command });
    // ...
});
```

---

## 5. Crash Dump Analysis

### 5.1 Accessing Crash Dumps

**Location:**
```
Windows: C:\Users\{user}\AppData\Local\waveterm-{instance}\Crashpad\completed\
macOS:   ~/Library/Application Support/waveterm-{instance}/Crashpad/completed/
Linux:   ~/.config/waveterm-{instance}/Crashpad/completed/
```

**Files:**
- `*.dmp` - Minidump files (binary)
- `*.meta` - Metadata (JSON)

### 5.2 Analyzing Dumps (Optional)

**Using Breakpad Tools:**
```bash
# Install minidump-stackwalk
npm install -g minidump

# Analyze dump
minidump-stackwalk crash.dmp
```

**Output includes:**
- Stack traces from all threads
- Register states
- Memory addresses
- Module list

### 5.3 Automatic Upload (Future)

```typescript
// In crash-reporter.ts
crashReporter.start({
    submitURL: "https://crash-reports.waveterm.dev/submit",
    uploadToServer: true, // Enable remote submission
    compress: true,
    // ... other options
});
```

---

## 6. Testing Strategy

### 6.1 Test Cases

**Test 1: JavaScript Exception**
```typescript
// Add to emain.ts for testing
electron.ipcMain.handle("test-js-crash", () => {
    throw new Error("Test JavaScript crash");
});
```
- ✅ Expect: Crash modal appears
- ✅ Expect: Error logged to waveapp.log
- ✅ Expect: Breadcrumbs saved

**Test 2: Native Crash (Simulated)**
```typescript
// Trigger renderer crash
electron.ipcMain.handle("test-renderer-crash", (event) => {
    event.sender.forcefullyCrashRenderer();
});
```
- ✅ Expect: Crash dump created in Crashpad/
- ✅ Expect: Recovery modal on next launch

**Test 3: Process Kill**
```bash
# Kill Wave from Task Manager or:
taskkill /F /IM Wave.exe
```
- ✅ Expect: Heartbeat becomes stale
- ✅ Expect: Recovery modal on next launch

**Test 4: GPU Crash**
```typescript
// Force GPU crash (Windows only)
process.crash(); // This will crash the main process
```
- ✅ Expect: GPU crash event fires
- ✅ Expect: Modal appears before restart

### 6.2 Manual Testing Checklist

- [ ] Install build with crash reporting
- [ ] Verify Crashpad directory is created
- [ ] Verify heartbeat file is updated every 5s
- [ ] Test JS exception crash
- [ ] Test native crash (force renderer crash)
- [ ] Test process kill (Task Manager)
- [ ] Restart and verify recovery modal
- [ ] Copy crash info and verify format
- [ ] Clear crash data and verify cleanup
- [ ] Test breadcrumbs are saved

---

## 7. User Experience

### 7.1 Normal Crash (JS Exception)

1. User performs action that triggers exception
2. **Crash Modal** appears immediately
3. User can:
   - Copy crash details
   - View logs
   - Report issue (GitHub pre-filled)
   - Close (app exits)
4. On next launch: **Recovery modal** (if dump exists)

### 7.2 Native Crash (GPU/Renderer)

1. GPU/Renderer crashes
2. Crash dump saved to Crashpad/
3. App may restart automatically (for GPU)
4. **On next launch:** Recovery modal appears
5. User sees:
   - Crash type
   - Timestamp
   - Recent actions (breadcrumbs)
   - Option to copy/clear

### 7.3 Silent Kill (Task Manager)

1. User kills process externally
2. Heartbeat stops updating
3. **On next launch:** Recovery modal appears
4. Modal shows "Process was terminated unexpectedly"
5. User can see last heartbeat time

---

## 8. Privacy & Security

### 8.1 Data Collected in Crash Dumps

**Included:**
- Stack traces (code execution path)
- Register values
- System information (OS, CPU, memory)
- Module list (loaded DLLs/libraries)
- WaveTerm version, platform, arch
- Recent events (breadcrumbs)

**NOT Included:**
- User credentials
- API keys
- File contents
- Terminal output
- SSH keys

### 8.2 Local-Only by Default

```typescript
crashReporter.start({
    uploadToServer: false, // Default: keep dumps local
});
```

User can opt-in to automatic reporting in settings:
```json
{
    "crash:auto-report": false,
    "crash:upload-dumps": false
}
```

---

## 9. Configuration Options

### 9.1 Settings

```json
{
    "crash:show-recovery-modal": true,
    "crash:save-breadcrumbs": true,
    "crash:max-breadcrumbs": 100,
    "crash:heartbeat-interval": 5000,
    "crash:auto-report": false,
    "crash:upload-dumps": false,
    "crash:clear-old-dumps-days": 30
}
```

### 9.2 Environment Variables

```bash
# Disable crash reporting (for development)
WAVE_CRASH_REPORTING=false Wave.exe

# Change crash directory
WAVE_CRASH_DIR=D:\MyCrashes Wave.exe

# Enable verbose crash logging
WAVE_CRASH_VERBOSE=true Wave.exe
```

---

## 10. Metrics & Monitoring

### 10.1 Crash Rate Tracking

Track crash rates over time:
```typescript
interface CrashMetrics {
    totalCrashes: number;
    crashesByType: Record<string, number>;
    lastCrash: string;
    crashRate: number; // crashes per hour
}
```

### 10.2 Alerting (Future)

If crash rate exceeds threshold:
- Show warning in app
- Suggest safe mode
- Offer to disable hardware acceleration

---

## 11. Implementation Checklist

### Phase 1: Native Crash Reporter (Week 1)
- [ ] Create `emain/crash-reporter.ts`
- [ ] Initialize crashReporter in emain.ts
- [ ] Test crash dump generation
- [ ] Verify Crashpad directory structure

### Phase 2: Real-Time Handlers (Week 1)
- [ ] Create `emain/crash-handlers.ts`
- [ ] Implement GPU crash handler
- [ ] Implement renderer crash handler
- [ ] Test all handlers with simulated crashes

### Phase 3: Heartbeat Monitor (Week 2)
- [ ] Create `emain/heartbeat.ts`
- [ ] Implement heartbeat writing
- [ ] Implement stale detection
- [ ] Test with process kill

### Phase 4: Breadcrumbs (Week 2)
- [ ] Create `emain/crash-breadcrumbs.ts`
- [ ] Add event tracking throughout app
- [ ] Test breadcrumb capture
- [ ] Verify file persistence

### Phase 5: Recovery Modal (Week 3)
- [ ] Create `emain/crash-recovery.ts`
- [ ] Implement crash detection on startup
- [ ] Implement recovery modal UI
- [ ] Test all crash types → recovery

### Phase 6: Integration & Testing (Week 3)
- [ ] Integrate all components in emain.ts
- [ ] Run full test suite
- [ ] Test on Windows, macOS, Linux
- [ ] Performance testing (overhead)

### Phase 7: Documentation (Week 4)
- [ ] Update BUILD.md with crash testing
- [ ] Add crash reporting docs
- [ ] Create user guide for crash data
- [ ] Document privacy policy

---

## 12. Future Enhancements

### 12.1 Crash Analytics Dashboard

Web dashboard showing:
- Crash trends over time
- Most common crash types
- Affected versions
- Platform distribution

### 12.2 Automatic Symbolication

Process crash dumps server-side with symbols to get human-readable stack traces.

### 12.3 Safe Mode

If multiple crashes detected:
```typescript
// Launch in safe mode
app.commandLine.appendSwitch('disable-gpu');
app.commandLine.appendSwitch('disable-software-rasterizer');
```

### 12.4 Crash Grouping

Group similar crashes by stack trace signature to identify patterns.

---

## 13. Success Metrics

- **Crash Detection Rate**: 100% of crashes detected (vs 0% currently)
- **User Reporting**: 80%+ of crashes have user context (breadcrumbs)
- **Time to Report**: <1 minute from crash to GitHub issue
- **False Positives**: <1% (clean shutdowns misidentified as crashes)
- **Performance Overhead**: <10ms per heartbeat, <1% CPU

---

## 14. References

- **Electron crashReporter**: https://www.electronjs.org/docs/latest/api/crash-reporter
- **Crashpad Documentation**: https://chromium.googlesource.com/crashpad/crashpad/
- **Process Events**: https://nodejs.org/api/process.html#process-events
- **Electron App Events**: https://www.electronjs.org/docs/latest/api/app#events
- **Related Specs**:
  - SPEC_CRASH_HANDLER_MODAL.md (JavaScript exceptions)
  - BUILD.md (Testing procedures)

---

**Status:** Ready for implementation
**Priority:** High - Critical for debugging and user support
**Estimated Effort:** 3-4 weeks (all phases)

---

## Appendix A: Quick Start Guide

**Minimum Implementation (1 day):**

1. Add crashReporter to `emain.ts`:
```typescript
import { crashReporter } from "electron";
crashReporter.start({ uploadToServer: false });
```

2. Add recovery check on startup:
```typescript
const crashesDir = crashReporter.getCrashesDirectory();
// Check for dumps and show modal if found
```

This alone would have caught the October 19 crash.
