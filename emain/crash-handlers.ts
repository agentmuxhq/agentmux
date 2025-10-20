// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import { showCrashDialog } from "./crash-handler";
import { writeCrashBreadcrumb } from "./crash-breadcrumbs";
import { log } from "./log";

/**
 * Initialize all real-time crash event handlers
 * This sets up handlers for:
 * - GPU process crashes
 * - Renderer process crashes
 *
 * Note: JavaScript exceptions (uncaughtException, unhandledRejection) are handled in emain.ts
 */
export function initCrashHandlers() {
    // 1. GPU process crashes
    electron.app.on("gpu-process-crashed", (event, killed) => {
        log("GPU process crashed, killed:", killed);
        writeCrashBreadcrumb("gpu-process-crashed", { killed });

        const choice = electron.dialog.showMessageBoxSync({
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
            defaultId: 0,
        });

        if (choice === 0) {
            log("User chose to restart after GPU crash");
            electron.app.relaunch();
        } else {
            log("User chose to quit after GPU crash");
        }

        electron.app.quit();
    });

    // 3. Renderer process crashes/gone
    electron.app.on("render-process-gone", (event, webContents, details) => {
        log("Renderer process gone:", details.reason, "exitCode:", details.exitCode);
        writeCrashBreadcrumb("render-process-gone", {
            reason: details.reason,
            exitCode: details.exitCode,
        });

        const reasonMessages: Record<string, string> = {
            "clean-exit": "Renderer exited cleanly (unexpected)",
            "abnormal-exit": "Renderer crashed",
            killed: "Renderer was killed",
            crashed: "Renderer crashed",
            oom: "Out of memory",
            "launch-failed": "Failed to launch renderer",
            "integrity-failure": "Code integrity check failed",
        };

        const message = reasonMessages[details.reason] || `Unknown reason: ${details.reason}`;

        const choice = electron.dialog.showMessageBoxSync({
            type: "error",
            title: "Renderer Process Crashed",
            message: "The rendering process has crashed.",
            detail:
                `Reason: ${message}\n` +
                `Exit Code: ${details.exitCode}\n\n` +
                (details.reason === "oom"
                    ? "This may be due to:\n" +
                      "• Too many tabs/windows open\n" +
                      "• Memory leak in a block\n" +
                      "• Large file operations\n\n"
                    : "") +
                "WaveTerm will restart.",
            buttons: ["Restart", "Quit"],
            defaultId: 0,
        });

        if (choice === 0) {
            log("User chose to restart after renderer crash");
            electron.app.relaunch();
        } else {
            log("User chose to quit after renderer crash");
        }

        electron.app.quit();
    });

    // Note: Child process errors (can add handlers for wavesrv here if needed)
    log("Crash handlers initialized (GPU, renderer process crashes)");
}
