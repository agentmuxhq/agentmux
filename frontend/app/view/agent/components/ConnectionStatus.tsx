// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * ConnectionStatus - Provider-aware connection state
 *
 * Displays auth UI appropriate for the current provider:
 * - Claude: OAuth button (browser flow)
 * - Gemini/Codex: API key input field
 */

import { useAtomValue, useSetAtom, atom as jotaiAtom } from "jotai";
import React, { memo, useCallback, useEffect, useState } from "react";
import { getApi } from "@/app/store/global";
import type { PrimitiveAtom } from "jotai";
import type { AuthState, UserInfo } from "../types";
import { PROVIDERS } from "../providers";

// Fallback atom for when providerConfigAtom is not provided
const fallbackProviderConfigAtom: PrimitiveAtom<ProviderConfig | null> = jotaiAtom<ProviderConfig | null>(null);

interface ConnectionStatusProps {
    authAtom: PrimitiveAtom<AuthState>;
    userInfoAtom: PrimitiveAtom<UserInfo | null>;
    providerConfigAtom?: PrimitiveAtom<ProviderConfig | null>;
}

export const ConnectionStatus: React.FC<ConnectionStatusProps> = memo(
    ({ authAtom, userInfoAtom, providerConfigAtom }) => {
        const authState = useAtomValue(authAtom);
        const userInfo = useAtomValue(userInfoAtom);
        const providerConfig = useAtomValue(providerConfigAtom ?? fallbackProviderConfigAtom);
        const setAuthState = useSetAtom(authAtom);
        const setUserInfo = useSetAtom(userInfoAtom);

        const currentProvider = providerConfig?.default_provider || "claude";
        const providerDef = PROVIDERS[currentProvider];
        const authType = providerDef?.authType || "oauth";

        // Listen for auth events from backend (OAuth flow)
        useEffect(() => {
            if (authType !== "oauth") return;

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
        }, [authType, setAuthState, setUserInfo]);

        const handleOAuthConnect = useCallback(async () => {
            try {
                console.log("[ConnectionStatus] Opening OAuth auth...");
                setAuthState({ status: "connecting" });
                await getApi().openClaudeCodeAuth();
            } catch (error) {
                console.error("[ConnectionStatus] Failed to open auth:", error);
                setAuthState({ status: "error", error: String(error) });
            }
        }, [setAuthState]);

        const handleDisconnect = useCallback(async () => {
            try {
                console.log("[ConnectionStatus] Disconnecting...");
                await getApi().clearProviderAuth(currentProvider);
                setAuthState({ status: "disconnected" });
                setUserInfo(null);
            } catch (error) {
                console.error("[ConnectionStatus] Failed to disconnect:", error);
                setAuthState({ status: "error", error: `Disconnect failed: ${String(error)}` });
            }
        }, [currentProvider, setAuthState, setUserInfo]);

        const handleRetry = useCallback(async () => {
            setAuthState({ status: "disconnected" });
            if (authType === "oauth") {
                await handleOAuthConnect();
            }
        }, [authType, setAuthState, handleOAuthConnect]);

        // Check auth status on mount and periodically
        useEffect(() => {
            let checkInterval: NodeJS.Timeout;
            let isMounted = true;

            const checkAuthStatus = async () => {
                try {
                    const status = await getApi().getProviderAuthStatus(currentProvider);
                    if (!isMounted) return;

                    if (status.status === "authenticated") {
                        setAuthState({ status: "connected" });
                    } else {
                        setAuthState({ status: "disconnected" });
                        setUserInfo(null);
                    }
                } catch (error) {
                    console.error("[ConnectionStatus] Failed to check auth status:", error);
                }
            };

            void checkAuthStatus();

            checkInterval = setInterval(() => {
                void checkAuthStatus();
            }, 5 * 60 * 1000);

            return () => {
                isMounted = false;
                if (checkInterval) clearInterval(checkInterval);
            };
        }, [currentProvider, setAuthState, setUserInfo]);

        if (authState.status === "connected" && userInfo) {
            return (
                <div className="agent-connection-status connected">
                    <div className="connection-icon">{"\u2713"}</div>
                    <div className="connection-info">
                        <div className="connection-label">
                            Connected to {providerDef?.displayName || currentProvider}
                        </div>
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
                    <div className="connection-icon">{"\u23F3"}</div>
                    <div className="connection-info">
                        <div className="connection-label">Connecting...</div>
                        <div className="connection-hint">
                            {authType === "oauth"
                                ? "Complete authentication in your browser"
                                : "Validating API key..."}
                        </div>
                    </div>
                </div>
            );
        }

        if (authState.status === "error") {
            return (
                <div className="agent-connection-status error">
                    <div className="connection-icon">{"\u26A0\uFE0F"}</div>
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

        // Disconnected state - show provider-appropriate auth UI
        if (authType === "api-key") {
            return <ApiKeyInput provider={currentProvider} providerDef={providerDef} onAuth={setAuthState} />;
        }

        // OAuth flow (Claude)
        return (
            <div className="agent-connection-status disconnected">
                <div className="connection-message">
                    <div className="connection-title">
                        Connect to {providerDef?.displayName || currentProvider}
                    </div>
                    <div className="connection-description">
                        Authenticate via browser to enable cloud-based conversations.
                    </div>
                </div>
                <button className="connection-connect-btn" onClick={handleOAuthConnect}>
                    <i className="fa fa-sign-in" /> Connect
                </button>
                <div className="connection-fallback">
                    Or use local mode with <code>{providerDef?.cliCommand || "cli"}</code> CLI (no auth required)
                </div>
            </div>
        );
    }
);

ConnectionStatus.displayName = "ConnectionStatus";

// --- API Key Input component for Gemini/Codex ---

const ApiKeyInput: React.FC<{
    provider: string;
    providerDef: any;
    onAuth: (state: AuthState) => void;
}> = memo(({ provider, providerDef, onAuth }) => {
    const [apiKey, setApiKey] = useState("");
    const [saving, setSaving] = useState(false);

    const handleSave = useCallback(async () => {
        if (!apiKey.trim()) return;
        setSaving(true);
        try {
            await getApi().setProviderAuth(provider, apiKey.trim());
            onAuth({ status: "connected" });
        } catch (error) {
            onAuth({ status: "error", error: `Failed to save API key: ${String(error)}` });
        } finally {
            setSaving(false);
        }
    }, [provider, apiKey, onAuth]);

    return (
        <div className="agent-connection-status disconnected">
            <div className="connection-message">
                <div className="connection-title">
                    {providerDef?.displayName || provider} API Key
                </div>
                <div className="connection-description">
                    Enter your API key to authenticate.
                </div>
            </div>
            <div className="connection-apikey-form">
                <input
                    type="password"
                    className="connection-apikey-input"
                    placeholder="Enter API key..."
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    onKeyDown={(e) => {
                        if (e.key === "Enter") void handleSave();
                    }}
                />
                <button
                    className="connection-connect-btn"
                    onClick={handleSave}
                    disabled={!apiKey.trim() || saving}
                >
                    {saving ? "Saving..." : "Save"}
                </button>
            </div>
            <div className="connection-fallback">
                Or use local mode with <code>{providerDef?.cliCommand || "cli"}</code> CLI (no auth required)
            </div>
        </div>
    );
});

ApiKeyInput.displayName = "ApiKeyInput";
