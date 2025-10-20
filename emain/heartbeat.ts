// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir } from "./platform";
import { log } from "./log";

const HEARTBEAT_INTERVAL = 5000; // 5 seconds
const HEARTBEAT_STALE_MS = 30000; // 30 seconds = stale
let heartbeatTimer: NodeJS.Timeout | null = null;

interface HeartbeatData {
    timestamp: number;
    pid: number;
    version?: string;
    cleanExit?: boolean;
}

export function getHeartbeatFilePath(): string {
    return path.join(getWaveDataDir(), "heartbeat.json");
}

/**
 * Start the heartbeat monitor
 * Writes a timestamp to a file every 5 seconds
 * This helps detect if the process was killed externally (Task Manager, SIGKILL, etc.)
 */
export function startHeartbeat() {
    const heartbeatFile = getHeartbeatFilePath();

    // Write initial heartbeat
    writeHeartbeat(heartbeatFile);

    // Update every 5 seconds
    heartbeatTimer = setInterval(() => {
        writeHeartbeat(heartbeatFile);
    }, HEARTBEAT_INTERVAL);

    log(`Heartbeat monitor started (interval: ${HEARTBEAT_INTERVAL}ms, file: ${heartbeatFile})`);
}

function writeHeartbeat(file: string) {
    try {
        let version = "unknown";
        try {
            version = require("../package.json").version;
        } catch (e) {
            // Ignore
        }

        const data: HeartbeatData = {
            timestamp: Date.now(),
            pid: process.pid,
            version,
        };

        fs.writeFileSync(file, JSON.stringify(data, null, 2), "utf-8");
    } catch (e) {
        // Don't throw - heartbeat failures should not crash the app
        log("Warning: Failed to write heartbeat:", e);
    }
}

/**
 * Stop the heartbeat monitor and mark as clean exit
 * Called when app is shutting down gracefully
 */
export function stopHeartbeat() {
    if (heartbeatTimer) {
        clearInterval(heartbeatTimer);
        heartbeatTimer = null;
    }

    // Write final heartbeat with clean exit flag
    const heartbeatFile = getHeartbeatFilePath();
    try {
        const data: HeartbeatData = {
            timestamp: Date.now(),
            pid: process.pid,
            cleanExit: true,
        };
        fs.writeFileSync(heartbeatFile, JSON.stringify(data, null, 2), "utf-8");
        log("Heartbeat monitor stopped (clean exit)");
    } catch (e) {
        log("Warning: Failed to write final heartbeat:", e);
    }
}

/**
 * Check if the previous session crashed based on heartbeat file
 * Returns crash info if detected, or null if clean exit
 */
export function checkForStaleCrash(): { crashed: boolean; reason: string; data?: any } {
    const heartbeatFile = getHeartbeatFilePath();

    if (!fs.existsSync(heartbeatFile)) {
        return { crashed: false, reason: "no-heartbeat-file" };
    }

    try {
        const content = fs.readFileSync(heartbeatFile, "utf-8");
        const data: HeartbeatData = JSON.parse(content);

        // Check if clean exit
        if (data.cleanExit) {
            return { crashed: false, reason: "clean-exit" };
        }

        // Check if heartbeat is stale (process killed)
        const now = Date.now();
        const age = now - data.timestamp;

        if (age > HEARTBEAT_STALE_MS) {
            return {
                crashed: true,
                reason: "stale-heartbeat",
                data: {
                    lastHeartbeat: new Date(data.timestamp).toISOString(),
                    ageSeconds: Math.floor(age / 1000),
                    ageMs: age,
                    pid: data.pid,
                    version: data.version,
                },
            };
        }

        // Heartbeat exists and is fresh (previous instance might still be running)
        return {
            crashed: false,
            reason: "fresh-heartbeat",
            data: {
                lastHeartbeat: new Date(data.timestamp).toISOString(),
                ageSeconds: Math.floor(age / 1000),
                pid: data.pid,
            },
        };
    } catch (e) {
        log("Warning: Failed to check heartbeat:", e);
        return { crashed: false, reason: "error-reading-heartbeat" };
    }
}

/**
 * Clear the heartbeat file
 */
export function clearHeartbeat() {
    const heartbeatFile = getHeartbeatFilePath();
    try {
        if (fs.existsSync(heartbeatFile)) {
            fs.unlinkSync(heartbeatFile);
            log("Heartbeat file cleared");
        }
    } catch (e) {
        log("Warning: Failed to clear heartbeat file:", e);
    }
}
