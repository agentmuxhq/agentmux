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

        // Initialize provider (loads config, sets up translator, starts CLI)
        this.initializeProvider();
    }

    /**
     * Initialize provider from persisted config.
     * If setup is complete, starts the CLI session with the configured provider.
     * If not, the view will show the SetupWizard.
     */
    private async initializeProvider(): Promise<void> {
        try {
            const config = await getApi().getProviderConfig();
            globalStore.set(this.atoms.providerConfigAtom, config);

            if (config.setup_complete) {
                this.startWithProvider(config);
            } else {
                console.log("[agent] Setup not complete, waiting for wizard");
            }
        } catch (error) {
            console.error("[agent] Failed to load provider config:", error);
            // Fall back to local terminal mode with default claude translator
            this.connectToTerminal();
        }
    }

    /**
     * Start the CLI session with the configured provider.
     * Sets up the translator, updates block metadata, and connects to terminal.
     */
    startWithProvider(config: ProviderConfig): void {
        const providerDef = getProvider(config.default_provider);
        if (!providerDef) {
            console.error("[agent] Unknown provider:", config.default_provider);
            return;
        }

        console.log("[agent] Starting with provider:", providerDef.displayName);

        // Update provider config atom so the view knows setup is complete
        globalStore.set(this.atoms.providerConfigAtom, config);

        // Create provider-specific translator
        this.translator = createTranslator(providerDef.outputFormat);

        // Update block metadata so the backend spawns the correct CLI
        this.updateBlockMeta(providerDef);

        // Update view icon and name based on provider
        globalStore.set(this.viewIcon as PrimitiveAtom<string>, providerDef.icon);
        globalStore.set(this.viewName as PrimitiveAtom<string>, providerDef.displayName);

        // Connect to terminal output
        this.connectToTerminal();

        // Force restart the controller with updated metadata
        RpcApi.ControllerResyncCommand(TabRpcClient, {
            tabid: globalStore.get(atoms.staticTabId),
            blockid: this.blockId,
            forcerestart: true,
        }).catch((e: any) => {
            console.error("[agent] Failed to start provider CLI:", e);
        });
    }

    /**
     * Update block meta to set the correct cmd and cmd:args for the provider.
     */
    private updateBlockMeta(providerDef: ProviderDefinition): void {
        const oref = WOS.makeORef("block", this.blockId);
        try {
            RpcApi.SetMetaCommand(TabRpcClient, {
                oref,
                meta: {
                    "cmd": providerDef.cliCommand,
                    "cmd:args": providerDef.defaultArgs,
                },
            }).catch((e: any) => {
                console.error("[agent] Failed to update block meta:", e);
            });
        } catch (e) {
            console.error("[agent] Failed to update block meta:", e);
        }
    }

    // --- Terminal connection ---

    private connectToTerminal(): void {
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

            // Parse NDJSON stream, translate through provider-specific translator,
            // then convert to document nodes
            const lines = text.split('\n').filter(line => line.trim());
            for (const line of lines) {
                try {
                    const rawEvent = JSON.parse(line);
                    // Translate raw CLI output to StreamEvent format
                    const streamEvents = this.translator.translate(rawEvent);
                    for (const se of streamEvents) {
                        const nodes = await this.parser.parseEvent(se);
                        if (nodes.length > 0) {
                            const currentDoc = globalStore.get(this.atoms.documentAtom);
                            globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
                        }
                    }
                } catch (err) {
                    console.warn("[agent] Failed to parse NDJSON line:", line, err);
                }
            }
        }
    }

    // --- User actions ---

    sendMessage = async (text: string): Promise<void> => {
        if (!text.trim()) return;

        if (this.useApiMode && this.apiClient) {
            // Use API mode
            await this.sendMessageViaAPI(text);
        } else {
            // Use local terminal mode
            const b64data = stringToBase64(text + "\n");
            RpcApi.ControllerInputCommand(TabRpcClient, {
                blockid: this.blockId,
                inputdata64: b64data,
            }).catch((e: any) => {
                console.error("[agent] Failed to send input:", e);
            });
        }
    };

    /**
     * Send message via Claude Code API
     * Streams response and appends nodes to document
     */
    private async sendMessageViaAPI(text: string): Promise<void> {
        if (!this.apiClient) {
            console.error("[agent] API client not initialized");
            return;
        }

        try {
            // Stream response from API
            for await (const event of this.apiClient.sendMessage(text, this.conversationId)) {
                // Parse event and append to document
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
        // TODO: Implement export logic
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
        globalStore.set(this.shellProcFullStatusAtom, null as any);
        globalStore.set(this.atoms.documentAtom, []); // Clear this instance's document
        RpcApi.ControllerResyncCommand(TabRpcClient, {
            tabid: globalStore.get(atoms.staticTabId),
            blockid: this.blockId,
            forcerestart: true,
        }).catch((e: any) => {
            console.error("[agent] Failed to restart process:", e);
        });
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
        }
    }

    // Defer focus to avoid infinite render loop in BlockFull's layout effect chain.
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
