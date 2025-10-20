# WaveTerm Crash Handler Modal Specification

**Version:** 1.0
**Date:** 2025-10-19
**Status:** Design Phase
**Target:** WaveTerm Fork v0.12.4+

---

## Executive Summary

Implement a user-facing crash handler modal that displays when WaveTerm encounters an uncaught exception. The modal will show the error details, stack trace, and system information with copy-to-clipboard functionality, allowing users to easily report crashes before the application exits.

**Core Problem:** Currently, when WaveTerm crashes due to an uncaught exception, the error is only logged to `waveapp.log` and the application silently quits, leaving users confused about what went wrong.

**Solution:** Display a modal dialog with comprehensive crash information that users can copy and paste for bug reports.

---

## 1. Background

### 1.1 Current Crash Handling

**Location:** `emain/emain.ts:655-672`

**Current Implementation:**
```typescript
process.on("uncaughtException", (error) => {
    if (caughtException) {
        return;
    }

    // Check if the error is related to QUIC protocol, if so, ignore
    if (error?.message?.includes("net::ERR_QUIC_PROTOCOL_ERROR")) {
        console.log("Ignoring QUIC protocol error:", error.message);
        console.log("Stack Trace:", error.stack);
        return;
    }

    caughtException = true;
    console.log("Uncaught Exception, shutting down: ", error);
    console.log("Stack Trace:", error.stack);
    // Optionally, handle cleanup or exit the app
    electronApp.quit();
});
```

**Issues:**
- No user feedback when crash occurs
- Users don't know what caused the crash
- No easy way to copy error details for bug reports
- Application exits immediately without explanation

### 1.2 Existing Dialog Patterns

The codebase already uses `electron.dialog.showMessageBoxSync()` in several places:
- Multi-instance dialogs (`emain/emain-wavesrv.ts:76`)
- Window close confirmations (`emain/emain-window.ts:276`)
- Workspace deletion confirmations (`emain/emain-window.ts:740`)
- ARM64 translation warnings (`emain/platform.ts:119`)

We will follow the same pattern for crash handling.

---

## 2. Proposed Solution

### 2.1 Crash Modal UI Design

**Modal Components:**
1. **Error Icon** - Standard error icon (type: "error")
2. **Title** - "WaveTerm Encountered an Error"
3. **Message** - Brief user-friendly description
4. **Details Section** - Expandable/scrollable text area with:
   - Error message
   - Stack trace
   - System information
   - Session information
5. **Buttons**:
   - "Copy to Clipboard" - Copy all crash details
   - "View Logs" - Open the log file location
   - "Report Issue" - Open GitHub issues page
   - "Close" - Exit the application

### 2.2 Information to Capture

**Crash Details:**
```typescript
interface CrashReport {
    timestamp: string;           // ISO 8601 timestamp
    error: {
        name: string;            // Error name (e.g., "TypeError")
        message: string;         // Error message
        stack: string;           // Full stack trace
    };
    system: {
        platform: string;        // OS (from unamePlatform)
        arch: string;            // Architecture (from unameArch)
        electronVersion: string; // Electron version
        waveVersion: string;     // Wave version
        waveBuildTime: number;   // Build timestamp
        nodeVersion: string;     // Node.js version
        chromeVersion: string;   // Chrome version
    };
    session: {
        dataDir: string;         // Wave data directory
        configDir: string;       // Wave config directory
        instanceId?: string;     // Multi-instance ID (if applicable)
        uptime: number;          // Process uptime in seconds
        windowCount: number;     // Number of open windows
    };
    logs: {
        logFile: string;         // Path to current log file
        recentLogs: string[];    // Last 50 lines from log
    };
}
```

### 2.3 Modal Layout

```
┌─────────────────────────────────────────────────────────────┐
│  ⚠️  WaveTerm Encountered an Error                          │
│                                                              │
│  WaveTerm has crashed due to an unexpected error. Please    │
│  copy the details below and report this issue.              │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ ERROR DETAILS                                          │ │
│  │                                                        │ │
│  │ Timestamp: 2025-10-19T14:23:45.123Z                   │ │
│  │                                                        │ │
│  │ Error: TypeError: Cannot read property 'foo' of null  │ │
│  │                                                        │ │
│  │ Stack Trace:                                          │ │
│  │   at Object.<anonymous> (/path/to/file.js:123:45)    │ │
│  │   at Module._compile (internal/modules/cjs:456:78)    │ │
│  │   ... (scrollable)                                    │ │
│  │                                                        │ │
│  │ System Information:                                   │ │
│  │   Platform: win32                                     │ │
│  │   Architecture: x64                                   │ │
│  │   Wave Version: v0.12.3                               │ │
│  │   Electron: 33.2.0                                    │ │
│  │   Node: 20.18.0                                       │ │
│  │                                                        │ │
│  │ Session Information:                                  │ │
│  │   Instance ID: test                                   │ │
│  │   Uptime: 3h 45m 12s                                  │ │
│  │   Open Windows: 2                                     │ │
│  │                                                        │ │
│  │ Log File: C:\Users\...\waveterm\waveapp.log          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│         [Copy to Clipboard] [View Logs] [Report Issue]      │
│                                            [Close] (default) │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Implementation Details

### 3.1 New File: `emain/crash-handler.ts`

Create a dedicated module for crash handling:

```typescript
// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir, getWaveConfigDir, unamePlatform, unameArch, getMultiInstanceInfo } from "./platform";
import { getWaveVersion } from "./emain-wavesrv";
import { getAllWaveWindows } from "./emain-window";
import { log } from "./log";

interface CrashReport {
    timestamp: string;
    error: {
        name: string;
        message: string;
        stack: string;
    };
    system: {
        platform: string;
        arch: string;
        electronVersion: string;
        waveVersion: string;
        waveBuildTime: number;
        nodeVersion: string;
        chromeVersion: string;
    };
    session: {
        dataDir: string;
        configDir: string;
        instanceId?: string;
        uptime: number;
        windowCount: number;
    };
    logs: {
        logFile: string;
        recentLogs: string[];
    };
}

function formatUptime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);

    if (hours > 0) {
        return `${hours}h ${minutes}m ${secs}s`;
    } else if (minutes > 0) {
        return `${minutes}m ${secs}s`;
    }
    return `${secs}s`;
}

function getRecentLogs(logFile: string, lines: number = 50): string[] {
    try {
        if (!fs.existsSync(logFile)) {
            return ["Log file not found"];
        }

        const content = fs.readFileSync(logFile, "utf-8");
        const allLines = content.split("\n");
        return allLines.slice(-lines);
    } catch (e) {
        return [`Error reading log file: ${e.message}`];
    }
}

function generateCrashReport(error: Error): CrashReport {
    const waveDataDir = getWaveDataDir();
    const waveConfigDir = getWaveConfigDir();
    const multiInstanceInfo = getMultiInstanceInfo();
    const waveVersion = getWaveVersion();
    const logFile = path.join(waveDataDir, "waveapp.log");

    return {
        timestamp: new Date().toISOString(),
        error: {
            name: error.name || "Error",
            message: error.message || "Unknown error",
            stack: error.stack || "No stack trace available",
        },
        system: {
            platform: unamePlatform,
            arch: unameArch,
            electronVersion: process.versions.electron,
            waveVersion: waveVersion.version,
            waveBuildTime: waveVersion.buildTime,
            nodeVersion: process.versions.node,
            chromeVersion: process.versions.chrome,
        },
        session: {
            dataDir: waveDataDir,
            configDir: waveConfigDir,
            instanceId: multiInstanceInfo.instanceId,
            uptime: process.uptime(),
            windowCount: getAllWaveWindows().length,
        },
        logs: {
            logFile: logFile,
            recentLogs: getRecentLogs(logFile),
        },
    };
}

function formatCrashReportForDisplay(report: CrashReport): string {
    return `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
WAVETERM CRASH REPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Timestamp: ${report.timestamp}

━━━ ERROR DETAILS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Error: ${report.error.name}: ${report.error.message}

Stack Trace:
${report.error.stack}

━━━ SYSTEM INFORMATION ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Platform:        ${report.system.platform}
Architecture:    ${report.system.arch}
Wave Version:    ${report.system.waveVersion}
Build Time:      ${new Date(report.system.waveBuildTime * 1000).toISOString()}
Electron:        ${report.system.electronVersion}
Node.js:         ${report.system.nodeVersion}
Chrome:          ${report.system.chromeVersion}

━━━ SESSION INFORMATION ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Data Directory:  ${report.session.dataDir}
Config Directory: ${report.session.configDir}
Instance ID:     ${report.session.instanceId || "default"}
Uptime:          ${formatUptime(report.session.uptime)}
Open Windows:    ${report.session.windowCount}

━━━ LOG FILE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Location: ${report.logs.logFile}

Recent Log Entries (last ${report.logs.recentLogs.length} lines):
${report.logs.recentLogs.join("\n")}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
`;
}

export async function showCrashDialog(error: Error): Promise<void> {
    try {
        // Generate crash report
        const crashReport = generateCrashReport(error);
        const reportText = formatCrashReportForDisplay(crashReport);

        // Log to file
        log("═════════════════════════════════════════════════════");
        log("CRASH REPORT");
        log("═════════════════════════════════════════════════════");
        log(reportText);
        log("═════════════════════════════════════════════════════");

        // Wait for app to be ready (in case crash happens during startup)
        await electron.app.whenReady();

        const { dialog, shell, clipboard } = electron;

        // Show dialog with crash details
        const dialogOpts: Electron.MessageBoxOptions = {
            type: "error",
            buttons: ["Copy to Clipboard", "View Logs", "Report Issue", "Close"],
            defaultId: 3, // Close is default
            cancelId: 3,
            title: "WaveTerm Encountered an Error",
            message: "WaveTerm has crashed due to an unexpected error.",
            detail: `${error.name}: ${error.message}\n\nClick "Copy to Clipboard" to copy full crash details for reporting this issue.\n\nLog file: ${crashReport.logs.logFile}`,
            noLink: true,
        };

        const choice = dialog.showMessageBoxSync(dialogOpts);

        switch (choice) {
            case 0: // Copy to Clipboard
                clipboard.writeText(reportText);

                // Show confirmation
                dialog.showMessageBoxSync({
                    type: "info",
                    buttons: ["OK"],
                    title: "Copied",
                    message: "Crash details copied to clipboard",
                    detail: "You can now paste this information when reporting the issue on GitHub.",
                });
                break;

            case 1: // View Logs
                // Open the log directory
                shell.showItemInFolder(crashReport.logs.logFile);
                break;

            case 2: // Report Issue
                // Open GitHub issues page with pre-filled template
                const issueTitle = encodeURIComponent(`Crash: ${error.message}`);
                const issueBody = encodeURIComponent(
                    `**Crash Report**\n\n\`\`\`\n${reportText}\n\`\`\`\n\n` +
                    `**Steps to Reproduce**\n1. \n2. \n3. \n\n` +
                    `**Expected Behavior**\n\n` +
                    `**Actual Behavior**\nApplication crashed with the error above.`
                );
                shell.openExternal(
                    `https://github.com/a5af/waveterm/issues/new?title=${issueTitle}&body=${issueBody}`
                );
                break;

            case 3: // Close
            default:
                // Just close
                break;
        }
    } catch (e) {
        // If crash dialog itself crashes, just log and exit
        console.error("Error showing crash dialog:", e);
        log("Error showing crash dialog:", e);
    }
}
```

### 3.2 Modify `emain/emain.ts`

Update the uncaught exception handler:

```typescript
// At top of file, add import:
import { showCrashDialog } from "./crash-handler";

// Replace lines 655-672 with:
let caughtException = false;
process.on("uncaughtException", async (error) => {
    if (caughtException) {
        return;
    }

    // Check if the error is related to QUIC protocol, if so, ignore (can happen with the updater)
    if (error?.message?.includes("net::ERR_QUIC_PROTOCOL_ERROR")) {
        console.log("Ignoring QUIC protocol error:", error.message);
        console.log("Stack Trace:", error.stack);
        return;
    }

    caughtException = true;
    console.log("Uncaught Exception, showing crash dialog:", error);
    console.log("Stack Trace:", error.stack);

    // Show crash dialog to user
    await showCrashDialog(error);

    // Exit application
    electronApp.quit();
});
```

### 3.3 Add Unhandled Rejection Handler

Also handle unhandled promise rejections:

```typescript
process.on("unhandledRejection", async (reason, promise) => {
    const error = reason instanceof Error ? reason : new Error(String(reason));
    console.log("Unhandled Promise Rejection:", error);
    console.log("Stack Trace:", error.stack);

    // Show crash dialog for unhandled rejections too
    await showCrashDialog(error);

    electronApp.quit();
});
```

---

## 4. User Experience Flow

### 4.1 Crash Scenario

1. **User Action** → Application encounters uncaught exception
2. **Crash Handler** → Catches exception, generates crash report
3. **Modal Display** → Shows error dialog with details
4. **User Options**:
   - **Copy to Clipboard** → Copies full report, shows confirmation
   - **View Logs** → Opens file explorer to log file location
   - **Report Issue** → Opens GitHub with pre-filled issue template
   - **Close** → Exits application
5. **Application Exit** → App quits gracefully after user dismisses dialog

### 4.2 User Benefits

- **Transparency**: Users know exactly what went wrong
- **Easy Reporting**: One-click copy to clipboard for bug reports
- **Quick Access**: Direct access to log files
- **Streamlined Issues**: Pre-filled GitHub issue template

---

## 5. Testing Strategy

### 5.1 Manual Testing

**Test Case 1: Trigger Crash During Runtime**
```typescript
// Add temporary test code in emain.ts
electron.ipcMain.on("test-crash", () => {
    throw new Error("Test crash from main process");
});
```
- Trigger crash from renderer
- Verify modal appears
- Test all buttons (Copy, View Logs, Report Issue, Close)

**Test Case 2: Crash During Startup**
```typescript
// Add temporary code early in appMain()
setTimeout(() => {
    throw new Error("Test startup crash");
}, 1000);
```
- Verify modal appears even during startup
- Verify all information is captured

**Test Case 3: Unhandled Promise Rejection**
```typescript
// Add temporary test code
electron.ipcMain.on("test-rejection", () => {
    Promise.reject(new Error("Test unhandled rejection"));
});
```
- Verify modal appears for rejections
- Verify stack trace is captured

### 5.2 Edge Cases

1. **Log File Missing**: Crash before log file created
2. **Long Stack Trace**: Verify scrolling works
3. **Unicode in Error**: Test special characters
4. **Multiple Windows**: Verify modal appears on correct window
5. **No Windows Open**: Verify modal can show without parent window
6. **Clipboard Failure**: Handle clipboard write errors

### 5.3 Acceptance Criteria

- ✅ Modal appears within 500ms of crash
- ✅ All crash details are captured and displayed
- ✅ Copy to Clipboard works and shows confirmation
- ✅ View Logs opens correct directory
- ✅ Report Issue opens GitHub with pre-filled template
- ✅ Close button exits application
- ✅ No crashes in crash handler itself (graceful degradation)

---

## 6. Configuration Options

### 6.1 Future Enhancement: Settings

Add optional settings for crash handling:

```json
{
    "crash:show-dialog": true,           // Show dialog (vs. just log)
    "crash:auto-report": false,          // Auto-submit crash reports
    "crash:include-recent-logs": true,   // Include logs in report
    "crash:log-lines": 50                // Number of log lines to include
}
```

### 6.2 Environment Variables

For testing and debugging:

```bash
# Disable crash dialog (just log and exit)
WAVE_CRASH_DIALOG=false Wave.exe

# Save crash report to file
WAVE_CRASH_SAVE_PATH=D:\crashes Wave.exe
```

---

## 7. Privacy Considerations

### 7.1 Data Included

**Safe to Include:**
- Error message and stack trace
- System information (OS, versions)
- Session metadata (uptime, window count)
- Log file path

**Never Include:**
- User credentials or API keys
- File contents or terminal output
- Personal identifiable information
- SSH keys or certificates

### 7.2 Log Sanitization

When including recent logs, filter out sensitive patterns:
- Authentication tokens
- API keys (common patterns)
- File paths with usernames (truncate)

Example:
```typescript
function sanitizeLog(log: string): string {
    return log
        .replace(/Bearer\s+[A-Za-z0-9_-]+/g, "Bearer [REDACTED]")
        .replace(/api[_-]?key[=:]\s*[^\s]+/gi, "api_key=[REDACTED]")
        .replace(/C:\\Users\\[^\\]+/g, "C:\\Users\\[USER]");
}
```

---

## 8. Implementation Checklist

### Phase 1: Core Implementation
- [ ] Create `emain/crash-handler.ts`
- [ ] Implement `generateCrashReport()` function
- [ ] Implement `formatCrashReportForDisplay()` function
- [ ] Implement `showCrashDialog()` function
- [ ] Update `emain/emain.ts` uncaughtException handler
- [ ] Add unhandledRejection handler

### Phase 2: Testing
- [ ] Add manual test triggers
- [ ] Test all button actions
- [ ] Test edge cases (no logs, startup crash, etc.)
- [ ] Verify clipboard functionality
- [ ] Test GitHub issue template

### Phase 3: Polish
- [ ] Add log sanitization
- [ ] Improve error message formatting
- [ ] Add configuration options
- [ ] Document crash handling in README

### Phase 4: Documentation
- [ ] Update BUILD.md with crash testing instructions
- [ ] Add crash reporting guide to docs
- [ ] Create GitHub issue template for crashes

---

## 9. Related Work

### 9.1 Similar Features

This crash handler is similar to:
- Multi-instance dialog (`emain/emain-wavesrv.ts:54-84`)
- ARM64 translation warning (`emain/platform.ts:109-128`)

### 9.2 Future Enhancements

1. **Crash Analytics**: Send anonymous crash reports to telemetry
2. **Auto-Recovery**: Attempt to restart with safe mode
3. **Session Restore**: Restore windows/tabs after crash
4. **Crash History**: Keep history of crashes for pattern detection

---

## 10. Success Metrics

- **User Awareness**: 100% of crashes show modal (vs. silent exit)
- **Bug Reports**: 50%+ increase in quality bug reports with full details
- **Time to Report**: Reduce time from crash to GitHub issue by 80%
- **User Satisfaction**: Positive feedback on crash transparency

---

## 11. References

- **Current Crash Handler**: `emain/emain.ts:655-672`
- **Dialog Examples**: `emain/emain-wavesrv.ts`, `emain/emain-window.ts`
- **Logging System**: `emain/log.ts`
- **Electron Dialog API**: https://www.electronjs.org/docs/latest/api/dialog
- **Clipboard API**: https://www.electronjs.org/docs/latest/api/clipboard

---

**Status:** Ready for implementation
**Priority:** High - Improves user experience and bug reporting
**Estimated Effort:** 1-2 days development + 1 day testing

---

## Appendix A: Example Crash Report Output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
WAVETERM CRASH REPORT
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Timestamp: 2025-10-19T14:23:45.123Z

━━━ ERROR DETAILS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Error: TypeError: Cannot read property 'workspaceId' of null

Stack Trace:
  at WaveBrowserWindow.getWorkspaceId (D:\Code\waveterm\dist\main\emain-window.js:145:32)
  at handleWSEvent (D:\Code\waveterm\dist\main\emain.js:125:18)
  at processTicksAndRejections (node:internal/process/task_queues:95:5)

━━━ SYSTEM INFORMATION ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Platform:        win32
Architecture:    x64
Wave Version:    v0.12.3
Build Time:      2025-10-18T20:15:30.000Z
Electron:        33.2.0
Node.js:         20.18.0
Chrome:          130.0.6723.59

━━━ SESSION INFORMATION ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Data Directory:  C:\Users\asafe\AppData\Local\waveterm\Data
Config Directory: C:\Users\asafe\.config\waveterm
Instance ID:     test
Uptime:          3h 45m 12s
Open Windows:    2

━━━ LOG FILE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Location: C:\Users\asafe\AppData\Local\waveterm\Data\waveapp.log

Recent Log Entries (last 50 lines):
2025-10-19 14:23:40.123 waveterm-app starting, data_dir=C:\Users\asafe\AppData\Local\waveterm\Data...
2025-10-19 14:23:41.456 wavesrv started successfully
2025-10-19 14:23:42.789 created new window workspace:abc123
2025-10-19 14:23:45.012 handleWSEvent electron:updateactivetab
2025-10-19 14:23:45.123 Uncaught Exception, showing crash dialog: TypeError: Cannot read property 'workspaceId' of null

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
