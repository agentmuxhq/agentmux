// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { BlockNodeModel } from "@/app/block/blocktypes";
import { RpcApi } from "@/app/store/wshclientapi";
import { TabRpcClient } from "@/app/store/wshrpcutil";
import { getFileSubject, waveEventSubscribe } from "@/app/store/wps";
import { atoms, globalStore, WOS, getApi } from "@/app/store/global";
import * as services from "@/app/store/services";
import { base64ToArray, stringToBase64 } from "@/util/util";
import { atom, Atom, PrimitiveAtom } from "jotai";
import React from "react";
import { Subscription } from "rxjs";
import { AgentViewWrapper } from "./agent-view";
import { ClaudeCodeStreamParser } from "./stream-parser";
import { ClaudeCodeApiClient } from "./api-client";
import {
    AgentAtoms,
    createAgentAtoms,
    createFilteredDocumentAtom,
    createDocumentStatsAtom,
    createToggleNodeCollapsed,
    createExpandAllNodes,
    createCollapseAllNodes,
    createClearDocument,
    createUpdateFilter,
} from "./state";
import { getProvider, type ProviderDefinition } from "./providers";
import { createTranslator } from "./providers/translator-factory";
import type { OutputTranslator } from "./providers/translator";

const TermFileName = "claude-code.jsonl";

export class AgentViewModel implements ViewModel {
    viewType = "agent";
    blockId: string;
    nodeModel: BlockNodeModel;
    blockAtom: Atom<Block>;

    viewIcon: Atom<string>;
    viewName: Atom<string>;
    viewText: Atom<string | HeaderElem[]>;
    viewComponent: ViewComponent;
    noPadding = atom(true);

    // Instance-scoped state atoms (NOT global!)
    agentIdValue: string;
    atoms: AgentAtoms;
    filteredDocumentAtom: Atom<DocumentNode[]>;
    documentStatsAtom: Atom<any>;
    toggleNodeCollapsed: any;
    expandAllNodes: any;
    collapseAllNodes: any;
    clearDocument: any;
    updateFilter: any;

    inputRef: React.RefObject<HTMLTextAreaElement>;
    shellProcStatusAtom: Atom<string>;
    shellProcExitCodeAtom: Atom<number>;

    // Internal
    private parser: ClaudeCodeStreamParser;
    private translator: OutputTranslator;
    private fileSubjectSub: Subscription | null = null;
    private fileSubjectRef: any = null;
    private procStatusUnsub: (() => void) | null = null;
    private shellProcFullStatusAtom: PrimitiveAtom<BlockControllerRuntimeStatus>;
    private apiClient: ClaudeCodeApiClient | null = null;
    private useApiMode: boolean = false;
    private conversationId: string;
    private currentProviderDef: ProviderDefinition | null = null;
    private loginMode: boolean = false; // true when running "claude auth login"

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        this.blockId = blockId;
        this.nodeModel = nodeModel;
        this.blockAtom = WOS.getWaveObjectAtom<Block>(`block:${blockId}`);
        this.viewComponent = AgentViewWrapper as any;

        // Create instance-scoped atoms for THIS widget
        this.agentIdValue = blockId;
        this.atoms = createAgentAtoms(blockId);

        // Create derived atoms
        this.filteredDocumentAtom = createFilteredDocumentAtom(
            this.atoms.documentAtom,
            this.atoms.documentStateAtom
        );
        this.documentStatsAtom = createDocumentStatsAtom(this.atoms.documentAtom);

        // Create action atoms
        this.toggleNodeCollapsed = createToggleNodeCollapsed(this.atoms.documentStateAtom);
        this.expandAllNodes = createExpandAllNodes(this.atoms.documentStateAtom);
        this.collapseAllNodes = createCollapseAllNodes(
            this.atoms.documentAtom,
            this.atoms.documentStateAtom
        );
        this.clearDocument = createClearDocument(
            this.atoms.documentAtom,
            this.atoms.documentStateAtom
        );
        this.updateFilter = createUpdateFilter(this.atoms.documentStateAtom);

        this.viewIcon = atom("sparkles");
        this.viewName = atom("Agent");

        this.inputRef = React.createRef<HTMLTextAreaElement>();
        this.shellProcFullStatusAtom = atom(null) as unknown as PrimitiveAtom<BlockControllerRuntimeStatus>;
        this.shellProcStatusAtom = atom((get) => {
            const fullStatus = get(this.shellProcFullStatusAtom);
            return fullStatus?.shellprocstatus ?? "init";
        });
        this.shellProcExitCodeAtom = atom((get) => {
            const fullStatus = get(this.shellProcFullStatusAtom);
            return fullStatus?.shellprocexitcode ?? 0;
        });

        // Header text: show document stats
        this.viewText = atom((get) => {
            const stats = get(this.documentStatsAtom);
            const parts: HeaderElem[] = [];
            if (stats.totalNodes > 0) {
                parts.push({
                    elemtype: "text",
                    text: `${stats.totalNodes} events`,
                });
            }
            return parts;
        });

        // Fetch initial controller status
        services.BlockService.GetControllerStatus(blockId).then((rts) => {
            this.updateShellProcStatus(rts);
        });

        // Subscribe to controller status events (process lifecycle)
        this.procStatusUnsub = waveEventSubscribe({
            eventType: "controllerstatus",
            scope: WOS.makeORef("block", blockId),
            handler: (event) => {
                const rts = event.data as BlockControllerRuntimeStatus;
                this.updateShellProcStatus(rts);
            },
        });

        this.parser = new ClaudeCodeStreamParser();
        this.translator = createTranslator("claude-stream-json"); // default
        this.conversationId = `conv-${blockId}`;

        // Initialize provider (loads config, checks auth, starts CLI)
        this.initializeProvider();
    }

    // =============================================
    // State Machine: SETUP → AUTH CHECK → SESSION
    // =============================================

    /**
     * SETUP_PENDING → CHECKING_AUTH or SETUP_WIZARD
     */
    private async initializeProvider(): Promise<void> {
        try {
            const config = await getApi().getProviderConfig();
            globalStore.set(this.atoms.providerConfigAtom, config);

            if (config.setup_complete) {
                this.prepareProvider(config);
                await this.checkAuth();
            } else {
                console.log("[agent] Setup not complete, waiting for wizard");
            }
        } catch (error) {
            console.error("[agent] Failed to load provider config:", error);
            this.connectToTerminal();
        }
    }

    /**
     * Configure translator and view for the selected provider (no process spawn yet).
     */
    private prepareProvider(config: ProviderConfig): void {
        const providerDef = getProvider(config.default_provider);
        if (!providerDef) {
            console.error("[agent] Unknown provider:", config.default_provider);
            return;
        }
        this.currentProviderDef = providerDef;
        this.translator = createTranslator(providerDef.outputFormat);
        globalStore.set(this.viewIcon as PrimitiveAtom<string>, providerDef.icon);
        globalStore.set(this.viewName as PrimitiveAtom<string>, providerDef.displayName);
    }

    /**
     * CHECKING_AUTH → AUTH_OK or AUTH_REQUIRED
     *
     * Runs `claude auth status --json` via Rust command.
     * For API-key providers (gemini/codex), skips to session start.
     */
    async checkAuth(): Promise<void> {
        if (!this.currentProviderDef) return;

        // API-key providers don't have a CLI auth check
        if (this.currentProviderDef.authType === "api-key") {
            globalStore.set(this.atoms.authAtom, { status: "connected" });
            this.startSession();
            return;
        }

        globalStore.set(this.atoms.authAtom, { status: "connecting" });
        console.log("[agent] Checking CLI auth status for:", this.currentProviderDef.id);

        try {
            const authStatus = await getApi().checkCliAuthStatus(this.currentProviderDef.id);
            console.log("[agent] Auth status:", authStatus);

            if (authStatus.logged_in) {
                // AUTH_OK → start session
                globalStore.set(this.atoms.authAtom, { status: "connected" });
                globalStore.set(this.atoms.userInfoAtom, {
                    email: authStatus.email || "",
                    name: authStatus.subscription_type || "",
                });
                this.startSession();
            } else {
                // AUTH_REQUIRED → show login UI
                globalStore.set(this.atoms.authAtom, { status: "disconnected" });
                console.log("[agent] Not logged in, waiting for user to authenticate");
            }
        } catch (error) {
            console.error("[agent] Auth check failed:", error);
            // Assume not logged in
            globalStore.set(this.atoms.authAtom, { status: "disconnected" });
        }
    }

    /**
     * LOGGING_IN: Spawn `claude auth login` in the PTY.
     * The user completes login in their browser.
     * When the process exits, we re-check auth.
     */
    startAuthLogin = (): void => {
        if (!this.currentProviderDef) return;

        console.log("[agent] Starting auth login for:", this.currentProviderDef.id);
        this.loginMode = true;
        globalStore.set(this.atoms.authAtom, { status: "connecting" });

        // Set block meta to run the auth login command
        const oref = WOS.makeORef("block", this.blockId);
        RpcApi.SetMetaCommand(TabRpcClient, {
            oref,
            meta: {
                "cmd": this.currentProviderDef.cliCommand,
                "cmd:args": this.currentProviderDef.authLoginCommand,
            },
        }).then(() => {
            // Connect to terminal to see login output
            this.connectToTerminal();
            // Start the login process
            return RpcApi.ControllerResyncCommand(TabRpcClient, {
                tabid: globalStore.get(atoms.staticTabId),
                blockid: this.blockId,
                forcerestart: true,
            });
        }).catch((e: any) => {
            console.error("[agent] Failed to start auth login:", e);
            globalStore.set(this.atoms.authAtom, {
                status: "error",
                error: `Failed to start login: ${String(e)}`,
            });
        });
    };

    /**
     * SESSION_STARTING: Set block meta to session args and spawn the CLI.
     */
    private startSession(): void {
        if (!this.currentProviderDef) return;

        this.loginMode = false;
        console.log("[agent] Starting session with:", this.currentProviderDef.displayName);
        globalStore.set(this.atoms.authAtom, { status: "connected" });

        // Set block meta to the session command
        const oref = WOS.makeORef("block", this.blockId);
        RpcApi.SetMetaCommand(TabRpcClient, {
            oref,
            meta: {
                "cmd": this.currentProviderDef.cliCommand,
                "cmd:args": this.currentProviderDef.defaultArgs,
            },
        }).then(() => {
            this.connectToTerminal();
            return RpcApi.ControllerResyncCommand(TabRpcClient, {
                tabid: globalStore.get(atoms.staticTabId),
                blockid: this.blockId,
                forcerestart: true,
            });
        }).catch((e: any) => {
            console.error("[agent] Failed to start session:", e);
        });
    }

    /**
     * Called from SetupWizard when user completes provider selection.
     * Triggers: SETUP_WIZARD → CHECKING_AUTH
     */
    startWithProvider(config: ProviderConfig): void {
        globalStore.set(this.atoms.providerConfigAtom, config);
        this.prepareProvider(config);
        this.checkAuth();
    }

    // --- Terminal connection ---

    private connectToTerminal(): void {
        // Avoid duplicate subscriptions
        if (this.fileSubjectSub) return;

        try {
            this.fileSubjectRef = getFileSubject(this.blockId, TermFileName);
            this.fileSubjectSub = this.fileSubjectRef.subscribe((msg: any) => {
                this.handleTerminalData(msg);
            });
            globalStore.set(this.atoms.processAtom, {
                ...globalStore.get(this.atoms.processAtom),
                status: "running",
            });
        } catch (e) {
            console.error("[agent] Failed to connect to terminal file subject:", e);
            globalStore.set(this.atoms.processAtom, {
                ...globalStore.get(this.atoms.processAtom),
                status: "failed",
            });
        }
    }

    private async handleTerminalData(msg: any): Promise<void> {
        if (msg.fileop === "truncate") {
            this.parser.reset();
            this.translator.reset();
            return;
        }
        if (msg.fileop === "append" && msg.data64) {
            const bytes = base64ToArray(msg.data64);
            const text = new TextDecoder().decode(bytes);

            // If in login mode, we just display the output as-is (not NDJSON)
            if (this.loginMode) {
                return;
            }

            // Parse NDJSON stream from the CLI session
            const lines = text.split('\n').filter(line => line.trim());
            for (const line of lines) {
                try {
                    const rawEvent = JSON.parse(line);
                    this.handleCliEvent(rawEvent);
                } catch (err) {
                    console.warn("[agent] Non-JSON line from CLI:", line);
                }
            }
        }
    }

    /**
     * Handle a parsed CLI event based on its top-level type.
     * Event types (verified from live CLI):
     *   system, stream_event, assistant, user, rate_limit_event, result
     */
    private async handleCliEvent(event: any): Promise<void> {
        switch (event.type) {
            case "system":
                this.handleSystemEvent(event);
                break;

            case "stream_event":
            case "assistant":
            case "user":
                // Translate through provider translator → stream parser → document nodes
                const streamEvents = this.translator.translate(event);
                for (const se of streamEvents) {
                    const nodes = await this.parser.parseEvent(se);
                    if (nodes.length > 0) {
                        const currentDoc = globalStore.get(this.atoms.documentAtom);
                        globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
                    }
                }
                break;

            case "rate_limit_event":
                this.handleRateLimitEvent(event);
                break;

            case "result":
                this.handleResultEvent(event);
                break;

            default:
                console.log("[agent] Unknown CLI event type:", event.type);
                break;
        }
    }

    /**
     * system.init = session started, auth confirmed.
     */
    private handleSystemEvent(event: any): void {
        if (event.subtype === "init") {
            console.log("[agent] Session initialized:", {
                session_id: event.session_id,
                model: event.model,
                version: event.claude_code_version,
                tools: event.tools?.length || 0,
            });
            globalStore.set(this.atoms.sessionIdAtom, event.session_id || "");
            globalStore.set(this.atoms.authAtom, { status: "connected" });
            globalStore.set(this.atoms.processAtom, {
                ...globalStore.get(this.atoms.processAtom),
                status: "running",
            });
        }
    }

    /**
     * rate_limit_event — log and potentially show to user.
     */
    private handleRateLimitEvent(event: any): void {
        const info = event.rate_limit_info;
        if (info?.status === "limited") {
            console.warn("[agent] Rate limited until:", new Date(info.resetsAt * 1000));
        }
    }

    /**
     * result = session complete (success or error).
     */
    private handleResultEvent(event: any): void {
        console.log("[agent] Session result:", {
            subtype: event.subtype,
            is_error: event.is_error,
            cost: event.total_cost_usd,
            turns: event.num_turns,
            duration: event.duration_ms,
        });

        if (event.is_error) {
            globalStore.set(this.atoms.processAtom, {
                ...globalStore.get(this.atoms.processAtom),
                status: "failed",
            });
        } else {
            globalStore.set(this.atoms.processAtom, {
                ...globalStore.get(this.atoms.processAtom),
                status: "idle",
            });
        }
    }

    // --- User actions ---

    sendMessage = async (text: string): Promise<void> => {
        if (!text.trim()) return;

        if (this.useApiMode && this.apiClient) {
            await this.sendMessageViaAPI(text);
        } else {
            const b64data = stringToBase64(text + "\n");
            RpcApi.ControllerInputCommand(TabRpcClient, {
                blockid: this.blockId,
                inputdata64: b64data,
            }).catch((e: any) => {
                console.error("[agent] Failed to send input:", e);
            });
        }
    };

    private async sendMessageViaAPI(text: string): Promise<void> {
        if (!this.apiClient) {
            console.error("[agent] API client not initialized");
            return;
        }
        try {
            for await (const event of this.apiClient.sendMessage(text, this.conversationId)) {
                const nodes = await this.parser.parseEvent(event);
                if (nodes.length > 0) {
                    const currentDoc = globalStore.get(this.atoms.documentAtom);
                    globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
                }
            }
        } catch (error) {
            console.error("[agent] Failed to send message via API:", error);
        }
    }

    exportDocument = (format: "markdown" | "html"): void => {
        const doc = globalStore.get(this.atoms.documentAtom);
        console.log("[agent] Export as", format, "- document has", doc.length, "nodes");
    };

    pauseAgent = (): void => {
        globalStore.set(this.atoms.processAtom, {
            ...globalStore.get(this.atoms.processAtom),
            status: "paused",
        });
    };

    resumeAgent = (): void => {
        globalStore.set(this.atoms.processAtom, {
            ...globalStore.get(this.atoms.processAtom),
            status: "running",
        });
    };

    killAgent = (): void => {
        RpcApi.ControllerInputCommand(TabRpcClient, {
            blockid: this.blockId,
            signame: "SIGINT",
        }).catch((e: any) => {
            console.error("[agent] Failed to send SIGINT:", e);
        });
        globalStore.set(this.atoms.processAtom, {
            ...globalStore.get(this.atoms.processAtom),
            status: "idle",
        });
    };

    restartAgent = (): void => {
        this.loginMode = false;
        globalStore.set(this.shellProcFullStatusAtom, null as any);
        globalStore.set(this.atoms.documentAtom, []);
        // Re-enter the auth check flow
        this.checkAuth();
    };

    private updateShellProcStatus(rts: BlockControllerRuntimeStatus): void {
        if (rts == null) return;
        const cur = globalStore.get(this.shellProcFullStatusAtom);
        if (cur == null || cur.version < rts.version) {
            globalStore.set(this.shellProcFullStatusAtom, rts);

            if (rts.shellprocstatus === "running") {
                globalStore.set(this.atoms.processAtom, {
                    ...globalStore.get(this.atoms.processAtom),
                    status: "running",
                });
            }

            // When login process finishes, re-check auth
            if (this.loginMode && rts.shellprocstatus === "done") {
                console.log("[agent] Login process exited, exit code:", rts.shellprocexitcode);
                this.loginMode = false;
                // Disconnect terminal so we can reconnect for session mode
                if (this.fileSubjectSub) {
                    this.fileSubjectSub.unsubscribe();
                    this.fileSubjectSub = null;
                }
                if (this.fileSubjectRef) {
                    this.fileSubjectRef.release();
                    this.fileSubjectRef = null;
                }
                // Re-check auth after login attempt
                this.checkAuth();
            }
        }
    }

    giveFocus(): boolean {
        if (this.inputRef.current) {
            requestAnimationFrame(() => this.inputRef.current?.focus());
            return true;
        }
        return false;
    }

    dispose(): void {
        if (this.fileSubjectSub) {
            this.fileSubjectSub.unsubscribe();
            this.fileSubjectSub = null;
        }
        if (this.fileSubjectRef) {
            this.fileSubjectRef.release();
            this.fileSubjectRef = null;
        }
        if (this.procStatusUnsub) {
            this.procStatusUnsub();
            this.procStatusUnsub = null;
        }
        this.parser.reset();
        this.translator.reset();
    }
}
