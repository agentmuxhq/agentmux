// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0s

import { getWebServerEndpoint } from "@/util/endpoints";
import { isRustBackend } from "@/util/tauri-rpc";
import { boundNumber, isBlank } from "@/util/util";
import { generate as generateCSS, parse as parseCSS, walk as walkCSS } from "css-tree";

/**
 * Get a streaming URL for a remote (or local) file.
 * In rust-backend mode, uses the `muxfile://` custom protocol.
 * In go-sidecar mode, uses the Go HTTP server endpoint.
 */
export function getStreamFileUrl(remotePath: string, connection?: string): string {
    const remoteUri = formatRemoteUri(remotePath, connection ?? "local");
    if (isRustBackend()) {
        const usp = new URLSearchParams();
        usp.set("path", remoteUri);
        if (connection != null) {
            usp.set("connection", connection);
        }
        return `muxfile://localhost/stream?${usp.toString()}`;
    }
    const usp = new URLSearchParams();
    usp.set("path", remoteUri);
    if (connection != null) {
        usp.set("connection", connection);
    }
    return `${getWebServerEndpoint()}/wave/stream-file?${usp.toString()}`;
}

/**
 * Get a streaming URL for a local file (no remote connection).
 * In rust-backend mode, uses the `muxfile://` custom protocol.
 * In go-sidecar mode, uses the Go HTTP server endpoint.
 */
export function getStreamLocalFileUrl(path: string, no404?: boolean): string {
    if (isRustBackend()) {
        const usp = new URLSearchParams();
        usp.set("path", path);
        if (no404) {
            usp.set("no404", "1");
        }
        return `muxfile://localhost/stream-local-file?${usp.toString()}`;
    }
    const usp = new URLSearchParams();
    usp.set("path", path);
    if (no404) {
        usp.set("no404", "1");
    }
    return `${getWebServerEndpoint()}/wave/stream-local-file?${usp.toString()}`;
}

function encodeFileURL(file: string) {
    if (isRustBackend()) {
        const remoteUri = formatRemoteUri(file, "local");
        const usp = new URLSearchParams();
        usp.set("path", remoteUri);
        usp.set("no404", "1");
        return `muxfile://localhost/stream?${usp.toString()}`;
    }
    const remoteUri = formatRemoteUri(file, "local");
    return `${getWebServerEndpoint()}/wave/stream-file?path=${encodeURIComponent(remoteUri)}&no404=1`;
}

export function processBackgroundUrls(cssText: string): string {
    if (isBlank(cssText)) {
        return null;
    }
    cssText = cssText.trim();
    if (cssText.endsWith(";")) {
        cssText = cssText.slice(0, -1);
    }
    const attrRe = /^background(-image)?\s*:\s*/i;
    cssText = cssText.replace(attrRe, "");
    const ast = parseCSS("background: " + cssText, {
        context: "declaration",
    });
    let hasUnsafeUrl = false;
    walkCSS(ast, {
        visit: "Url",
        enter(node) {
            const originalUrl = node.value.trim();
            if (
                originalUrl.startsWith("http:") ||
                originalUrl.startsWith("https:") ||
                originalUrl.startsWith("data:")
            ) {
                return;
            }
            // allow file:/// urls (if they are absolute)
            if (originalUrl.startsWith("file://")) {
                const path = originalUrl.slice(7);
                if (!path.startsWith("/")) {
                    console.log(`Invalid background, contains a non-absolute file URL: ${originalUrl}`);
                    hasUnsafeUrl = true;
                    return;
                }
                const newUrl = encodeFileURL(path);
                node.value = newUrl;
                return;
            }
            // allow absolute paths
            if (originalUrl.startsWith("/") || originalUrl.startsWith("~/") || /^[a-zA-Z]:(\/|\\)/.test(originalUrl)) {
                const newUrl = encodeFileURL(originalUrl);
                node.value = newUrl;
                return;
            }
            hasUnsafeUrl = true;
            console.log(`Invalid background, contains an unsafe URL scheme: ${originalUrl}`);
        },
    });
    if (hasUnsafeUrl) {
        return null;
    }
    const rtnStyle = generateCSS(ast);
    if (rtnStyle == null) {
        return null;
    }
    return rtnStyle.replace(/^background:\s*/, "");
}

export function computeBgStyleFromMeta(meta: MetaType, defaultOpacity: number = null): React.CSSProperties {
    const bgAttr = meta?.["bg"];
    if (isBlank(bgAttr)) {
        return null;
    }
    try {
        const processedBg = processBackgroundUrls(bgAttr);
        const rtn: React.CSSProperties = {};
        rtn.background = processedBg;
        rtn.opacity = boundNumber(meta["bg:opacity"], 0, 1) ?? defaultOpacity;
        if (!isBlank(meta?.["bg:blendmode"])) {
            rtn.backgroundBlendMode = meta["bg:blendmode"];
        }
        return rtn;
    } catch (e) {
        console.error("error processing background", e);
        return null;
    }
}

export function formatRemoteUri(path: string, connection: string): string {
    connection = connection ?? "local";
    // TODO: We need a better way to handle s3 paths
    let retVal: string;
    if (connection.startsWith("aws:")) {
        retVal = `${connection}:s3://${path ?? ""}`;
    } else {
        retVal = `wsh://${connection}/${path}`;
    }
    return retVal;
}
