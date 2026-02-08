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
        if ((window as any).api?.feLog) {
            (window as any).api.feLog(`[${level}] ${args.join(' ')}`);
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
        const isTauri = !!(window as any).__TAURI__;
        log("INFO", "Is Tauri:", isTauri);

        if (isTauri) {
            log("INFO", "Initializing Tauri API...");
            await setupTauriApi();
            log("INFO", "✅ Tauri API initialized successfully");
            log("INFO", "window.api available:", !!(window as any).api);
        } else {
            log("INFO", "Running in Electron mode, skipping Tauri init");
        }

        // Now dynamically import wave.ts
        log("INFO", "Loading main application (wave.ts)...");
        await import("./wave");
        log("INFO", "✅ Main application loaded successfully");

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
