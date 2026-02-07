// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Tauri initialization module.
// This must run BEFORE any code that accesses window.api (getApi()).
//
// In Electron, window.api is populated by the preload script (contextBridge).
// In Tauri, we populate it ourselves using invoke/listen.

import { buildTauriApi, initTauriApi, isTauri } from "@/util/tauri-api";

/**
 * Initialize the Tauri API shim if running inside Tauri.
 * Sets window.api to a Tauri-backed implementation of ElectronApi.
 *
 * This MUST be awaited before importing wave.ts or any module
 * that calls getApi() at the top level.
 */
export async function setupTauriApi(): Promise<void> {
    if (!isTauri()) {
        return; // Running in Electron, preload.ts handles window.api
    }

    // Pre-fetch all cached values from Rust backend
    await initTauriApi();

    // Build the API shim and install it on window
    const api = buildTauriApi();
    (window as any).api = api;

    console.log("[tauri-init] window.api installed");
}
