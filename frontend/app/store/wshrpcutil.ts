// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { wpsReconnectHandler } from "@/app/store/wps";
import { TabClient } from "@/app/store/tabrpcclient";
import { makeTabRouteId, WshRouter } from "@/app/store/wshrouter";
import { getWSServerEndpoint } from "@/util/endpoints";
import { isRustBackend, sendTauriRpc } from "@/util/tauri-rpc";
import { addWSReconnectHandler, globalWS, initGlobalWS, WSControl } from "./ws";
import { DefaultRouter, setDefaultRouter } from "./wshrpcutil-base";
import { getApi } from "./global";

let TabRpcClient: TabClient;

function initWshrpc(tabId: string): WSControl | null {
    if (isRustBackend()) {
        return initTauriRpc(tabId);
    }
    return initWebSocketRpc(tabId);
}

/**
 * Initialize RPC via Tauri IPC (rust-backend mode).
 * No WebSocket needed — messages go through invoke("rpc_request").
 */
function initTauriRpc(tabId: string): null {
    const router = new WshRouter(new TauriUpstreamProxy());
    setDefaultRouter(router);

    TabRpcClient = new TabClient(makeTabRouteId(tabId));
    DefaultRouter.registerRoute(TabRpcClient.routeId, TabRpcClient);

    console.log("[wshrpc] Initialized Tauri IPC RPC for tab:", tabId);
    return null;
}

/**
 * Initialize RPC via WebSocket (go-sidecar mode).
 * Original behavior — connects to ws://127.0.0.1:8877/ws.
 */
function initWebSocketRpc(tabId: string): WSControl {
    const router = new WshRouter(new UpstreamWshRpcProxy());
    setDefaultRouter(router);
    const handleFn = (event: WSEventType) => {
        DefaultRouter.recvRpcMessage(event.data);
    };

    // For Tauri: Get auth key from API and pass it to WebSocket
    // Browser WebSocket doesn't support custom headers, so wsutil.ts will
    // append it as a query parameter: ws://endpoint?authkey=xxx
    const authKey = getApi().getAuthKey();
    const eoOpts = authKey ? { authKey } : undefined;

    initGlobalWS(getWSServerEndpoint(), tabId, handleFn, eoOpts);
    globalWS.connectNow("connectWshrpc");
    TabRpcClient = new TabClient(makeTabRouteId(tabId));
    DefaultRouter.registerRoute(TabRpcClient.routeId, TabRpcClient);
    addWSReconnectHandler(() => {
        DefaultRouter.reannounceRoutes();
    });
    addWSReconnectHandler(wpsReconnectHandler);
    return globalWS;
}

/**
 * Tauri IPC upstream proxy — sends RPC messages via invoke()
 * instead of WebSocket. Response is routed back through the router.
 */
class TauriUpstreamProxy implements AbstractWshClient {
    recvRpcMessage(msg: RpcMessage): void {
        // Fire-and-forget for messages without reqid (routeannounce, events, etc.)
        if (!msg.reqid && msg.command !== "eventsub" && msg.command !== "eventpublish") {
            sendTauriRpc(msg).catch((e) => {
                console.error("[tauri-rpc] fire-and-forget failed:", e);
            });
            return;
        }

        // For messages expecting a response, send and route the response back
        sendTauriRpc(msg).then((response) => {
            if (response && (response.resid || response.data != null)) {
                DefaultRouter.recvRpcMessage(response);
            }
        }).catch((e) => {
            // Send error response back through the router
            if (msg.reqid) {
                DefaultRouter.recvRpcMessage({
                    resid: msg.reqid,
                    error: String(e),
                } as RpcMessage);
            }
        });
    }
}

class UpstreamWshRpcProxy implements AbstractWshClient {
    recvRpcMessage(msg: RpcMessage): void {
        const wsMsg: WSRpcCommand = { wscommand: "rpc", message: msg };
        globalWS?.pushMessage(wsMsg);
    }
}

export { DefaultRouter, initWshrpc, TabRpcClient };
export { initElectronWshrpc, sendRpcCommand, sendRpcResponse, shutdownWshrpc } from "./wshrpcutil-base";
