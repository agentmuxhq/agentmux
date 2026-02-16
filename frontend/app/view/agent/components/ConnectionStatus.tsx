// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * ConnectionStatus - Shows Claude Code connection state
 *
 * Displays either:
 * 1. "Connect" button when disconnected
 * 2. User info when connected
 */

import { useAtomValue } from "jotai";
import React, { memo, useCallback } from "react";
import type { PrimitiveAtom } from "jotai";
import type { AuthState, UserInfo } from "../types";

interface ConnectionStatusProps {
    authAtom: PrimitiveAtom<AuthState>;
    userInfoAtom: PrimitiveAtom<UserInfo | null>;
}

export const ConnectionStatus: React.FC<ConnectionStatusProps> = memo(({ authAtom, userInfoAtom }) => {
    const authState = useAtomValue(authAtom);
    const userInfo = useAtomValue(userInfoAtom);

    const handleConnect = useCallback(async () => {
        try {
            // This will be implemented in Phase 3 (backend)
            // For now, it's a placeholder that would call:
            // await getApi().openClaudeCodeAuth();
            console.log("[ConnectionStatus] Connect clicked - auth flow not yet implemented");
        } catch (error) {
            console.error("[ConnectionStatus] Failed to open auth:", error);
        }
    }, []);

    if (authState.status === "connected" && userInfo) {
        return (
            <div className="agent-connection-status connected">
                <div className="connection-icon">✓</div>
                <div className="connection-info">
                    <div className="connection-label">Connected to Claude Code</div>
                    <div className="connection-email">{userInfo.email}</div>
                </div>
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
                <button className="connection-retry-btn" onClick={handleConnect}>
                    Retry
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
