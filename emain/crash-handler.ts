// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir, getWaveConfigDir, unamePlatform, unameArch, getMultiInstanceInfo } from "./platform";
import { getWaveVersion } from "./emain-wavesrv";
import { getAllWaveWindows } from "./emain-window";
import { log } from "./log";
import { getCrashBreadcrumbs, formatBreadcrumbs } from "./crash-breadcrumbs";

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
    breadcrumbs: string;
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
    const breadcrumbs = getCrashBreadcrumbs();

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
        breadcrumbs: formatBreadcrumbs(breadcrumbs, 20),
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

━━━ RECENT EVENTS ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

${report.breadcrumbs}

━━━ LOG FILE ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Location: ${report.logs.logFile}

Recent Log Entries (last ${report.logs.recentLogs.length} lines):
${report.logs.recentLogs.join("\n")}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
`;
}

/**
 * Show crash dialog for JavaScript exceptions
 * This is called when an uncaught exception or unhandled rejection occurs
 */
export async function showCrashDialog(error: Error): Promise<void> {
    try {
        // Generate crash report
        const crashReport = generateCrashReport(error);
        const reportText = formatCrashReportForDisplay(crashReport);

        // Log to file
        log("═════════════════════════════════════════════════════");
        log("CRASH REPORT - JAVASCRIPT EXCEPTION");
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
            detail:
                `${error.name}: ${error.message}\n\n` +
                `Click "Copy to Clipboard" to copy full crash details for reporting this issue.\n\n` +
                `Log file: ${crashReport.logs.logFile}`,
            noLink: true,
        };

        const choice = dialog.showMessageBoxSync(dialogOpts);

        switch (choice) {
            case 0: // Copy to Clipboard
                clipboard.writeText(reportText);
                log("Crash details copied to clipboard");

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
                log("Opened log file location in file explorer");
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
                shell.openExternal(`https://github.com/a5af/waveterm/issues/new?title=${issueTitle}&body=${issueBody}`);
                log("Opened GitHub issues page");
                break;

            case 3: // Close
            default:
                log("User closed crash dialog");
                // Just close
                break;
        }
    } catch (e) {
        // If crash dialog itself crashes, just log and exit
        console.error("Error showing crash dialog:", e);
        log("Error showing crash dialog:", e);
    }
}
