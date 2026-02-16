// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * ConnectionStatus - Shows Claude Code connection state
 *
 * Displays either:
 * 1. "Connect" button when disconnected
 * 2. User info when connected
 */

import { useAtomValue, useSetAtom } from "jotai";
import React, { memo, useCallback, useEffect } from "react";
import { getApi } from "@/app/store/global";
import type { PrimitiveAtom } from "jotai";
import type { AuthState, UserInfo } from "../types";

interface ConnectionStatusProps {
    authAtom: PrimitiveAtom<AuthState>;
    userInfoAtom: PrimitiveAtom<UserInfo | null>;
}

export const ConnectionStatus: React.FC<ConnectionStatusProps> = memo(({ authAtom, userInfoAtom }) => {
    const authState = useAtomValue(authAtom);
    const userInfo = useAtomValue(userInfoAtom);
    const setAuthState = useSetAtom(authAtom);
    const setUserInfo = useSetAtom(userInfoAtom);

    // Listen for auth events from backend
    useEffect(() => {
        const unlistenStart = getApi().listen("claude-code-auth-started", () => {
            console.log("[ConnectionStatus] Auth started");
            setAuthState({ status: "connecting" });
        });

        const unlistenSuccess = getApi().listen("claude-code-auth-success", (event: any) => {
            console.log("[ConnectionStatus] Auth success:", event.payload);
            const payload = event.payload;
            setAuthState({ status: "connected" });
            setUserInfo({
                email: payload.email || "user@example.com",
                name: payload.name,
            });
        });

        const unlistenError = getApi().listen("claude-code-auth-error", (event: any) => {
            console.error("[ConnectionStatus] Auth error:", event.payload);
            setAuthState({
                status: "error",
                error: event.payload?.message || "Authentication failed",
            });
        });

        return () => {
            unlistenStart.then((fn) => fn());
            unlistenSuccess.then((fn) => fn());
            unlistenError.then((fn) => fn());
        };
    }, [setAuthState, setUserInfo]);

    const handleConnect = useCallback(async () => {
        try {
            console.log("[ConnectionStatus] Opening Claude Code auth...");
            setAuthState({ status: "connecting" });
            await getApi().openClaudeCodeAuth();
        } catch (error) {
            console.error("[ConnectionStatus] Failed to open auth:", error);
            setAuthState({
                status: "error",
                error: String(error),
            });
        }
    }, [setAuthState]);

    const handleDisconnect = useCallback(async () => {
        try {
            console.log("[ConnectionStatus] Disconnecting from Claude Code...");
            await getApi().disconnectClaudeCode();
            setAuthState({ status: "disconnected" });
            setUserInfo(null);
        } catch (error) {
            console.error("[ConnectionStatus] Failed to disconnect:", error);
            setAuthState({
                status: "error",
                error: `Disconnect failed: ${String(error)}`,
            });
        }
    }, [setAuthState, setUserInfo]);

    const handleRetry = useCallback(async () => {
        setAuthState({ status: "disconnected" });
        await handleConnect();
    }, [setAuthState, handleConnect]);

    // Check auth status on mount and periodically
    useEffect(() => {
        let checkInterval: NodeJS.Timeout;

        const checkAuthStatus = async () => {
            try {
                const status = await getApi().getClaudeCodeAuth();
                if (status.connected) {
                    setAuthState({ status: "connected" });
                    setUserInfo({
                        email: status.email || "user@example.com",
                        name: undefined,
                    });
                }
            } catch (error) {
                console.error("[ConnectionStatus] Failed to check auth status:", error);
            }
        };

        // Initial check
        checkAuthStatus();

        // Periodic check every 5 minutes to detect token expiration
        checkInterval = setInterval(checkAuthStatus, 5 * 60 * 1000);

        return () => {
            if (checkInterval) {
                clearInterval(checkInterval);
            }
        };
    }, [setAuthState, setUserInfo]);

    if (authState.status === "connected" && userInfo) {
        return (
            <div className="agent-connection-status connected">
                <div className="connection-icon">✓</div>
                <div className="connection-info">
                    <div className="connection-label">Connected to Claude Code</div>
                    <div className="connection-email">{userInfo.email}</div>
                </div>
                <button className="connection-disconnect-btn" onClick={handleDisconnect} title="Disconnect">
                    <i className="fa fa-sign-out" />
                </button>
            </div>
        );
    }

    if (authState.status === "connecting") {
        return (
            <div className="agent-connection-status connecting">
                <div className="connection-icon">⏳</div>
                <div className="connection-info">
                    <div className="connection-label">Connecting...</div>
                    <div className="connection-hint">Complete authentication in your browser</div>
                </div>
            </div>
        );
    }

    if (authState.status === "error") {
        return (
            <div className="agent-connection-status error">
                <div className="connection-icon">⚠️</div>
                <div className="connection-info">
                    <div className="connection-label">Connection Failed</div>
                    {authState.error && <div className="connection-error">{authState.error}</div>}
                </div>
                <button className="connection-retry-btn" onClick={handleRetry}>
                    <i className="fa fa-refresh" /> Retry
                </button>
            </div>
        );
    }

    // Disconnected state (default)
    return (
        <div className="agent-connection-status disconnected">
            <div className="connection-message">
                <div className="connection-title">Connect to Claude Code</div>
                <div className="connection-description">
                    Use the Claude Code API for cloud-based conversations. Requires authentication.
                </div>
            </div>
            <button className="connection-connect-btn" onClick={handleConnect}>
                <i className="fa fa-sign-in" /> Connect
            </button>
            <div className="connection-fallback">
                Or use local mode with <code>claude</code> CLI (no auth required)
            </div>
        </div>
    );
});

ConnectionStatus.displayName = "ConnectionStatus";
