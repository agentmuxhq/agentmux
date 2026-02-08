// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Tauri bootstrap with verbose logging.
// This is the true entry point for Tauri builds.

import { setupTauriApi } from "./tauri-init";

// Deep verbose logging
const log = (level: string, ...args: any[]) => {
    const timestamp = new Date().toISOString();
    console.log(`[${timestamp}] [${level}]`, ...args);

    // Also log to backend if available
    try {
        if ((window as any).api?.sendLog) {
            (window as any).api.sendLog(`[${level}] ${args.join(' ')}`);
        }
    } catch (e) {
        // Ignore if backend not ready
    }
};

(window as any).debugLog = log;

async function bootstrap() {
    try {
        log("INFO", "=== Tauri Bootstrap Starting ===");
        log("INFO", "User Agent:", navigator.userAgent);
        log("INFO", "Location:", window.location.href);

        // Check if we're in Tauri
        const isTauriRuntime = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
        log("INFO", "Is Tauri:", isTauriRuntime);

        if (isTauriRuntime) {
            log("INFO", "Initializing Tauri API...");
            await setupTauriApi();
            log("INFO", "✅ Tauri API initialized successfully");
            log("INFO", "window.api available:", !!(window as any).api);

            // Verify critical methods exist
            const api = (window as any).api;
            log("INFO", "API methods check:");
            log("INFO", "  - getAuthKey:", typeof api?.getAuthKey);
            log("INFO", "  - onContextMenuClick:", typeof api?.onContextMenuClick);
            log("INFO", "  - showContextMenu:", typeof api?.showContextMenu);
        } else {
            log("INFO", "Running in Electron mode, skipping Tauri init");
        }

        // Now dynamically import wave.ts
        log("INFO", "Loading main application (wave.ts)...");
        try {
            await import("./wave");
            log("INFO", "✅ Main application loaded successfully");
        } catch (waveError) {
            log("ERROR", "Failed to load wave.ts:", waveError);
            log("ERROR", "Wave error stack:", (waveError as Error).stack);
            throw waveError;
        }

    } catch (error) {
        log("ERROR", "❌ Bootstrap failed:", error);
        log("ERROR", "Stack:", (error as Error).stack);

        // Show error to user
        document.body.innerHTML = `
            <div style="padding: 20px; font-family: monospace; color: red;">
                <h1>WaveMux Failed to Start</h1>
                <pre>${error}</pre>
                <pre>${(error as Error).stack}</pre>
                <p>Check the browser console (F12) for more details.</p>
            </div>
        `;
        throw error;
    }
}

// Start bootstrap
bootstrap();
