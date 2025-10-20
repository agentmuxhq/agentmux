// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import * as fs from "fs";
import * as path from "path";
import { checkForStaleCrash } from "./heartbeat";
import { loadBreadcrumbsFromFile, formatBreadcrumbs, clearBreadcrumbs } from "./crash-breadcrumbs";
import { getCrashesDirectory } from "./crash-reporter";
import { log } from "./log";

export interface CrashInfo {
    type: "stale-heartbeat" | "crash-dump" | "unclean-shutdown";
    timestamp?: string;
    crashDumps?: string[];
    breadcrumbs?: any[];
    heartbeatData?: any;
}

/**
 * Check for evidence of a previous crash
 * Called on app startup to detect crashes from last session
 */
export function checkForPreviousCrash(): CrashInfo | null {
    const results: CrashInfo = {
        type: "unclean-shutdown",
        crashDumps: [],
        breadcrumbs: [],
    };

    let foundCrash = false;

    // 1. Check heartbeat for stale/killed process
    try {
        const heartbeatCheck = checkForStaleCrash();
        if (heartbeatCheck.crashed) {
            foundCrash = true;
            results.type = "stale-heartbeat";
            results.heartbeatData = heartbeatCheck.data;
            results.timestamp = heartbeatCheck.data?.lastHeartbeat;
            log("Previous crash detected via stale heartbeat:", heartbeatCheck.data);
        }
    } catch (e) {
        log("Error checking heartbeat:", e);
    }

    // 2. Check for crash dumps (native crashes)
    try {
        const crashesDir = getCrashesDirectory();
        if (fs.existsSync(crashesDir)) {
            // Check both 'completed' and 'reports' subdirectories
            const completedDir = path.join(crashesDir, "completed");
            const reportsDir = path.join(crashesDir, "reports");

            const dumps: string[] = [];

            if (fs.existsSync(completedDir)) {
                const files = fs.readdirSync(completedDir);
                files.forEach((f) => {
                    if (f.endsWith(".dmp")) {
                        dumps.push(path.join(completedDir, f));
                    }
                });
            }

            if (fs.existsSync(reportsDir)) {
                const files = fs.readdirSync(reportsDir);
                files.forEach((f) => {
                    if (f.endsWith(".dmp")) {
                        dumps.push(path.join(reportsDir, f));
                    }
                });
            }

            if (dumps.length > 0) {
                foundCrash = true;
                results.type = "crash-dump";
                results.crashDumps = dumps;

                // Get timestamp of newest dump
                const stats = fs.statSync(dumps[0]);
                results.timestamp = stats.mtime.toISOString();

                log(`Previous crash detected: found ${dumps.length} crash dump(s)`);
            }
        }
    } catch (e) {
        log("Error checking for crash dumps:", e);
    }

    // 3. Load breadcrumbs from previous session
    try {
        results.breadcrumbs = loadBreadcrumbsFromFile();
        if (results.breadcrumbs.length > 0) {
            log(`Loaded ${results.breadcrumbs.length} breadcrumbs from previous session`);
        }
    } catch (e) {
        log("Error loading breadcrumbs:", e);
        results.breadcrumbs = [];
    }

    return foundCrash ? results : null;
}

/**
 * Show recovery modal to user after detecting a crash
 */
export async function showRecoveryModal(crashInfo: CrashInfo): Promise<void> {
    try {
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
            defaultId: 2, // Continue is default
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
                log("Crash info copied to clipboard");

                dialog.showMessageBoxSync({
                    type: "info",
                    buttons: ["OK"],
                    title: "Copied",
                    message: "Crash information copied to clipboard",
                    detail: "You can now paste this information when reporting the issue on GitHub.",
                });
                break;

            case 1: // Clear Crashes
                clearCrashData(crashInfo);
                log("Crash data cleared by user");

                dialog.showMessageBoxSync({
                    type: "info",
                    buttons: ["OK"],
                    title: "Cleared",
                    message: "Crash data has been cleared.",
                });
                break;

            case 2: // Continue
            default:
                log("User chose to continue without clearing crash data");
                // Just continue, leave crash data for later analysis
                break;
        }
    } catch (e) {
        log("Error showing recovery modal:", e);
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
${crashInfo.crashDumps.map((d) => `  - ${d}`).join("\n")}

`;
    }

    if (crashInfo.breadcrumbs && crashInfo.breadcrumbs.length > 0) {
        details += `━━━ RECENT EVENTS (Last ${Math.min(crashInfo.breadcrumbs.length, 20)}) ━━━━━━━━━━━━━━━━━━━━

${formatBreadcrumbs(crashInfo.breadcrumbs, 20)}

`;
    }

    details += `━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

To report this crash:
1. Copy this information (already in clipboard if you clicked "Copy")
2. Go to https://github.com/a5af/waveterm/issues/new
3. Paste the crash information
4. Describe what you were doing before the crash

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`;

    return details;
}

function clearCrashData(crashInfo: CrashInfo) {
    // Clear crash dumps
    if (crashInfo.crashDumps) {
        for (const dump of crashInfo.crashDumps) {
            try {
                fs.unlinkSync(dump);
                log(`Deleted crash dump: ${dump}`);
            } catch (e) {
                log(`Failed to delete crash dump ${dump}:`, e);
            }
        }
    }

    // Clear breadcrumbs
    try {
        clearBreadcrumbs();
    } catch (e) {
        log("Failed to clear breadcrumbs:", e);
    }
}
