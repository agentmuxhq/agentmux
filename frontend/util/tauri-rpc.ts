// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Tauri IPC RPC transport — replaces WebSocket communication
// when running in rust-backend mode.
//
// Instead of ws://127.0.0.1:8877/ws, RPC messages are sent via
// invoke("rpc_request") and responses returned directly.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

let _isRustBackend = false;
let _wpsUnlisten: UnlistenFn | null = null;

/**
 * Set whether we're in rust-backend mode.
 * Called during initTauriApi() based on backend-ready payload.
 */
export function setRustBackendMode(enabled: boolean) {
    _isRustBackend = enabled;
    if (enabled) {
        console.log("[tauri-rpc] Rust backend mode enabled — using Tauri IPC for RPC");
    }
}

/**
 * Check if we're in rust-backend mode (Tauri IPC instead of WebSocket).
 */
export function isRustBackend(): boolean {
    return _isRustBackend;
}

/**
 * Send an RPC message via Tauri IPC and get the response.
 * Replaces WebSocket send/receive for rust-backend mode.
 */
export async function sendTauriRpc(msg: RpcMessage): Promise<RpcMessage | null> {
    try {
        const response = await invoke<any>("rpc_request", { msg });
        return response;
    } catch (e) {
        console.error("[tauri-rpc] rpc_request failed:", e);
        if (msg.reqid) {
            return {
                resid: msg.reqid,
                error: String(e),
            } as RpcMessage;
        }
        return null;
    }
}

/**
 * Call a backend service via Tauri IPC.
 * Replaces HTTP POST to /wave/service for rust-backend mode.
 */
export async function callTauriService(
    service: string,
    method: string,
    args: any[],
    uiContext?: any
): Promise<any> {
    try {
        const response = await invoke<any>("service_request", {
            service,
            method,
            args,
            uiContext: uiContext ?? null,
        });
        return response;
    } catch (e) {
        throw new Error(`service ${service}.${method} failed: ${e}`);
    }
}

/**
 * Start listening for WPS events from the Rust backend via Tauri emit.
 * Routes received events through the existing handleWaveEvent handler.
 * Must be called after the frontend event system is initialized.
 */
export async function initTauriWpsEventListener() {
    if (_wpsUnlisten) {
        _wpsUnlisten();
    }
    // Lazy import to avoid circular dependency
    const { handleWaveEvent } = await import("@/app/store/wps");
    _wpsUnlisten = await listen<any>("wps-event", (tauriEvent) => {
        const waveEvent = tauriEvent.payload;
        if (waveEvent && waveEvent.event) {
            handleWaveEvent(waveEvent);
        }
    });
    console.log("[tauri-rpc] WPS event listener initialized");
}

/**
 * Set terminal size for a block via Tauri IPC.
 * Replaces the WebSocket `setblocktermsize` command in rust-backend mode.
 */
export async function setBlockTermSize(blockId: string, rows: number, cols: number): Promise<void> {
    try {
        await invoke("set_block_term_size", { blockId, rows, cols });
    } catch (e) {
        console.error("[tauri-rpc] set_block_term_size failed:", e);
    }
}
