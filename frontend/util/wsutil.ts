// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import type { WebSocket as NodeWebSocketType } from "ws";

let NodeWebSocket: typeof NodeWebSocketType = null;

if (typeof window === "undefined") {
    // Necessary to avoid issues with Rollup: https://github.com/websockets/ws/issues/2057
    import("ws")
        .then((ws) => (NodeWebSocket = ws.default))
        .catch((e) => {
            console.log("Error importing 'ws':", e);
        });
}

type ComboWebSocket = NodeWebSocketType | WebSocket;

function newWebSocket(url: string, headers: { [key: string]: string }): ComboWebSocket {
    if (NodeWebSocket) {
        // Node.js WebSocket (backend): supports headers
        return new NodeWebSocket(url, { headers });
    } else {
        // Browser WebSocket (Electron/Tauri): does not support headers
        // For Tauri compatibility, append auth key as query parameter
        // (Electron injects headers via session.webRequest, so this is a no-op for Electron)
        let finalUrl = url;
        if (headers && headers["X-AuthKey"]) {
            const separator = url.includes("?") ? "&" : "?";
            finalUrl = `${url}${separator}authkey=${encodeURIComponent(headers["X-AuthKey"])}`;
        }
        return new WebSocket(finalUrl);
    }
}

export { newWebSocket };
export type { ComboWebSocket as WebSocket };
