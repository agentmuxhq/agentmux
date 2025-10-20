// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import * as path from "path";
import { getWaveDataDir } from "./platform";
import { log } from "./log";

/**
 * Initialize Electron's native crash reporter
 * This captures crashes in the main process, renderer processes, and GPU process
 * Crash dumps are saved locally to Crashpad/completed/ directory
 */
export function initCrashReporter() {
    const waveDataDir = getWaveDataDir();

    // Get package version safely
    let version = "unknown";
    try {
        version = require("../package.json").version;
    } catch (e) {
        log("Warning: Could not load package.json version for crash reporter");
    }

    try {
        electron.crashReporter.start({
            productName: "WaveTerm",
            companyName: "CommandLine",
            submitURL: "", // Local-only for now, no remote submission
            uploadToServer: false, // Don't send to remote server
            compress: true,
            ignoreSystemCrashHandler: false,
            rateLimit: false,
            globalExtra: {
                // Additional metadata included in all crashes
                waveVersion: version,
                platform: process.platform,
                arch: process.arch,
                electronVersion: process.versions.electron,
                nodeVersion: process.versions.node,
                chromeVersion: process.versions.chrome,
            },
        });

        const crashesDir = electron.crashReporter.getCrashesDirectory();
        log("Crash reporter initialized");
        log(`Crash dumps will be saved to: ${crashesDir}`);
        log(`Crash reporting enabled: uploadToServer=${false}`);
    } catch (e) {
        // Don't let crash reporter initialization crash the app
        log("Error initializing crash reporter (non-fatal):", e);
    }
}

/**
 * Get the directory where crash dumps are stored
 */
export function getCrashesDirectory(): string {
    try {
        return electron.crashReporter.getCrashesDirectory();
    } catch (e) {
        // Fallback if crashReporter not initialized
        return path.join(getWaveDataDir(), "Crashpad", "completed");
    }
}

/**
 * Get information about the crash reporter status
 */
export function getCrashReporterStatus(): {
    enabled: boolean;
    crashesDir: string;
    uploadsEnabled: boolean;
} {
    try {
        const crashesDir = electron.crashReporter.getCrashesDirectory();
        return {
            enabled: true,
            crashesDir,
            uploadsEnabled: false, // We don't upload by default
        };
    } catch (e) {
        return {
            enabled: false,
            crashesDir: "",
            uploadsEnabled: false,
        };
    }
}
