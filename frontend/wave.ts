// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { App } from "@/app/app";
import {
    globalRefocus,
    registerControlShiftStateUpdateHandler,
    registerElectronReinjectKeyHandler,
    registerGlobalKeys,
} from "@/app/store/keymodel";
import { modalsModel } from "@/app/store/modalmodel";
import { ClientService, WindowService, WorkspaceService } from "@/app/store/services";
import { RpcApi } from "@/app/store/wshclientapi";
import { initWshrpc, TabRpcClient } from "@/app/store/wshrpcutil";
import { loadMonaco } from "@/app/view/codeeditor/codeeditor";
import { getLayoutModelForStaticTab } from "@/layout/index";
import {
    atoms,
    countersClear,
    countersPrint,
    getApi,
    globalStore,
    initGlobal,
    initGlobalEventSubs,
    loadConnStatus,
    pushFlashError,
    pushNotification,
    removeNotificationById,
    subscribeToConnEvents,
} from "@/app/store/global";
import * as WOS from "@/app/store/wos";
import { loadFonts } from "@/util/fontutil";
import { setKeyUtilPlatform } from "@/util/keyutil";
import { createElement } from "react";
import { createRoot } from "react-dom/client";


const platform = getApi().getPlatform();

const appVersion = getApi().getAboutModalDetails().version;

document.title = `AgentMux ${appVersion}`;
let savedInitOpts: AgentMuxInitOpts = null;

// Update window title with instance ID if running in multi-instance mode
async function updateWindowTitleWithInstanceID() {
    try {
        // Only in Tauri
        if (!(window as any).__TAURI_INTERNALS__) {
            return;
        }

        const { invoke } = await import("@tauri-apps/api/core");
        const { readTextFile, exists } = await import("@tauri-apps/plugin-fs");
        const { getCurrentWindow } = await import("@tauri-apps/api/window");

        const dataDir = await invoke<string>("get_data_dir");
        // Use platform-agnostic path joining
        const instanceIDPath = dataDir.endsWith("/") || dataDir.endsWith("\\")
            ? `${dataDir}instance-id.txt`
            : `${dataDir}/instance-id.txt`;

        const fileExists = await exists(instanceIDPath);
        if (fileExists) {
            const instanceID = await readTextFile(instanceIDPath);
            if (instanceID && instanceID.trim()) {
                const newTitle = `AgentMux ${appVersion} [${instanceID.trim()}]`;
                document.title = newTitle;

                // Also update Tauri window title
                const window = getCurrentWindow();
                await window.setTitle(newTitle);

                console.log(`[multi-instance] Running as: ${instanceID.trim()}`);
            }
        }
    } catch (e) {
        // Ignore errors - instance file may not exist for default instance
        console.log("[multi-instance] No instance ID file found (default instance)");
    }
}

// Call after a short delay to allow backend to write the file
setTimeout(updateWindowTitleWithInstanceID, 1000);


(window as any).WOS = WOS;
(window as any).globalStore = globalStore;
(window as any).globalAtoms = atoms;
(window as any).RpcApi = RpcApi;
(window as any).isFullScreen = false;
(window as any).countersPrint = countersPrint;
(window as any).countersClear = countersClear;
(window as any).getLayoutModelForStaticTab = getLayoutModelForStaticTab;
(window as any).pushFlashError = pushFlashError;
(window as any).pushNotification = pushNotification;
(window as any).removeNotificationById = removeNotificationById;
(window as any).modalsModel = modalsModel;

function updateZoomFactor(zoomFactor: number) {
    document.documentElement.style.setProperty("--zoomfactor", String(zoomFactor));
    document.documentElement.style.setProperty("--zoomfactor-inv", String(1 / zoomFactor));
}

/**
 * Initialize AgentMux in Tauri mode by fetching client/window/workspace/tab data
 * from backend, verifying objects exist, and creating missing ones if needed.
 * This mirrors Electron's relaunchBrowserWindows() pattern.
 */
async function initTauriWave(): Promise<void> {

    try {
        // Get client data (like Electron's relaunchBrowserWindows)
        const clientData = await ClientService.GetClientData();

        let windowId = clientData.windowids?.[0];

        // If no windows exist, create one
        if (!windowId) {
            const newWindow = await WindowService.CreateWindow(null, "");
            windowId = newWindow.oid;
        }

        // Verify window exists
        let windowData = await WindowService.GetWindow(windowId);

        if (!windowData) {
            windowData = await WindowService.CreateWindow(null, "");
            windowId = windowData.oid;
        }


        // Get workspace
        let workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);

        if (!workspace) {
            // Workspace missing → recreate entire window (like Electron does)
            await WindowService.CloseWindow(windowData.oid, false);
            windowData = await WindowService.CreateWindow(null, "");
            workspace = await WorkspaceService.GetWorkspace(windowData.workspaceid);
        }


        // Get active tab ID
        const tabId = workspace.activetabid ||
                     workspace.tabids?.[0] ||
                     workspace.pinnedtabids?.[0] ||
                     "";

        if (!tabId) {
            throw new Error("No tab found in workspace");
        }


        // Create complete init options with ALL valid IDs
        const initOpts: AgentMuxInitOpts = {
            clientId: clientData.oid,
            windowId: windowData.oid,
            tabId: tabId,
            activate: true,
            primaryTabStartup: true,
        };


        // Initialize wave (this will render the UI)
        await initWaveWrap(initOpts);

        // Show the window now that it's fully initialized (Tauri starts hidden)
        try {
            const { getCurrent } = await import("@tauri-apps/api/window");
            const currentWindow = getCurrent();
            await currentWindow.show();
            await currentWindow.setFocus();
        } catch (showError) {
            console.warn("[initTauriWave] Failed to show window:", showError);
        }

    } catch (error) {
        console.error("[initTauriWave] Initialization failed:", error);
        pushFlashError("Failed to initialize AgentMux: " + String(error));
        // Show window even on error so user can see the error message
        try {
            const { getCurrent } = await import("@tauri-apps/api/window");
            await getCurrent().show();
        } catch {}
    }
}

/**
 * Initialize a new (non-main) Tauri window by creating new backend objects.
 * Unlike initTauriWave() which reuses existing Window/Workspace/Tab,
 * this creates a fresh set for the new window.
 */
async function initTauriNewWindow(): Promise<void> {
    try {
        getApi().sendLog("[initTauriNewWindow] Creating new backend objects");

        // Get client data (reuse existing client)
        const clientData = await ClientService.GetClientData();
        getApi().sendLog(`[initTauriNewWindow] Client ID: ${clientData.oid}`);

        // Create NEW window (not reuse)
        const newWindow = await WindowService.CreateWindow(null, "");
        getApi().sendLog(`[initTauriNewWindow] Created Window ID: ${newWindow.oid}`);

        // Get the workspace that was auto-created with the window
        const workspace = await WorkspaceService.GetWorkspace(newWindow.workspaceid);
        if (!workspace) {
            throw new Error("Workspace not created with new window");
        }
        getApi().sendLog(`[initTauriNewWindow] Workspace ID: ${workspace.oid}`);

        // Get the active tab ID from the workspace
        const tabId = workspace.activetabid ||
                     workspace.tabids?.[0] ||
                     workspace.pinnedtabids?.[0] ||
                     "";

        if (!tabId) {
            throw new Error("No tab found in new workspace");
        }
        getApi().sendLog(`[initTauriNewWindow] Tab ID: ${tabId}`);

        // Create complete init options with NEW IDs
        const initOpts: AgentMuxInitOpts = {
            clientId: clientData.oid,
            windowId: newWindow.oid,
            tabId: tabId,
            activate: true,
            primaryTabStartup: false, // Not primary (main window is primary)
        };

        getApi().sendLog("[initTauriNewWindow] Initializing wave with new objects");

        // Initialize wave (this will render the UI)
        await initWaveWrap(initOpts);

        getApi().sendLog("[initTauriNewWindow] ✅ New window initialized successfully");

        // Show the window now that it's initialized
        try {
            const { getCurrent } = await import("@tauri-apps/api/window");
            const currentWindow = getCurrent();
            await currentWindow.show();
            await currentWindow.setFocus();
            getApi().sendLog("[initTauriNewWindow] Window shown and focused");
        } catch (showError) {
            console.warn("[initTauriNewWindow] Failed to show window:", showError);
        }

    } catch (error) {
        console.error("[initTauriNewWindow] Initialization failed:", error);
        getApi().sendLog(`[initTauriNewWindow] ❌ Error: ${error}`);
        pushFlashError("Failed to initialize new window: " + String(error));
        // Show error UI instead of grey screen
        document.body.style.visibility = "visible";
        document.body.style.opacity = "1";
    }
}

async function initBare() {
    getApi().sendLog("Init Bare");
    document.body.style.visibility = "hidden";
    document.body.style.opacity = "0";
    document.body.classList.add("is-transparent");

    // Check if we're in Tauri mode
    const isTauri = typeof (window as any).__TAURI_INTERNALS__ !== "undefined";
    getApi().sendLog(`Init Bare - Tauri mode: ${isTauri}`);

    // Electron uses onAgentMuxInit callback (backend emits wave-init event)
    // Tauri handles initialization in frontend after backend is ready
    if (!isTauri) {
        getApi().onAgentMuxInit(initWaveWrap);
    }
    setKeyUtilPlatform(platform);
    loadFonts();
    updateZoomFactor(getApi().getZoomFactor());
    getApi().onZoomFactorChange((zoomFactor) => {
        updateZoomFactor(zoomFactor);
    });

    // Initialize zoom state
    import("@/app/store/zoom").then(({ loadZoom }) => {
        loadZoom(globalStore);
    });

    // Use Promise.race to add a timeout fallback for fonts.ready
    // In Tauri, fonts.ready might not resolve promptly
    const fontsPromise = document.fonts.ready;
    const timeoutPromise = new Promise(resolve => setTimeout(resolve, 2000));

    Promise.race([fontsPromise, timeoutPromise]).then(async () => {
        getApi().sendLog("Init Bare Done");
        getApi().setWindowInitStatus("ready");

        // In Tauri mode, handle initialization in frontend
        if (isTauri) {
            getApi().sendLog("Starting Tauri initialization");
            try {
                // Check if this is a new window or the main window
                const isMain = await getApi().isMainWindow();
                getApi().sendLog(`Window type: ${isMain ? "main" : "new window"}`);

                if (isMain) {
                    // Main window: standard initialization
                    await initTauriWave();
                } else {
                    // New window: create backend objects first
                    const label = await getApi().getWindowLabel();
                    getApi().sendLog(`Initializing new window: ${label}`);
                    await initTauriNewWindow();
                }
            } catch (error) {
                console.error("[initBare] Tauri initialization failed:", error);
                getApi().sendLog(`Tauri init error: ${error}`);
            }
        }

    });
}

// Handle both cases: DOM not yet loaded, or already loaded (Tauri dynamic import)
if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initBare);
} else {
    // DOM already loaded (e.g., when dynamically imported from tauri-bootstrap)
    initBare();
}

async function initWaveWrap(initOpts: AgentMuxInitOpts) {
    try {
        if (savedInitOpts) {
            await reinitWave();
            return;
        }
        savedInitOpts = initOpts;
        await initWave(initOpts);
    } catch (e) {
        getApi().sendLog("Error in initWave " + e.message + "\n" + e.stack);
        console.error("Error in initWave", e);
    } finally {
        document.body.style.visibility = null;
        document.body.style.opacity = null;
        document.body.classList.remove("is-transparent");
    }
}

async function reinitWave() {
    console.log("Reinit Wave");
    getApi().sendLog("Reinit Wave");

    // We use this hack to prevent a flicker of the previously-hovered tab when this view was last active.
    document.body.classList.add("nohover");
    requestAnimationFrame(() =>
        setTimeout(() => {
            document.body.classList.remove("nohover");
        }, 100)
    );

    await WOS.reloadWaveObject<Client>(WOS.makeORef("client", savedInitOpts.clientId));
    const waveWindow = await WOS.reloadWaveObject<WaveWindow>(WOS.makeORef("window", savedInitOpts.windowId));
    const ws = await WOS.reloadWaveObject<Workspace>(WOS.makeORef("workspace", waveWindow.workspaceid));
    const initialTab = await WOS.reloadWaveObject<Tab>(WOS.makeORef("tab", savedInitOpts.tabId));
    await WOS.reloadWaveObject<LayoutState>(WOS.makeORef("layout", initialTab.layoutstate));
    reloadAllWorkspaceTabs(ws);
    document.title = `AgentMux ${appVersion} - ${initialTab.name}`; // TODO update with tab name change
    getApi().setWindowInitStatus("wave-ready");
    globalStore.set(atoms.reinitVersion, globalStore.get(atoms.reinitVersion) + 1);
    globalStore.set(atoms.updaterStatusAtom, getApi().getUpdaterStatus());
    setTimeout(() => {
        globalRefocus();
    }, 50);
}

function reloadAllWorkspaceTabs(ws: Workspace) {
    if (ws == null || (!ws.tabids?.length && !ws.pinnedtabids?.length)) {
        return;
    }
    ws.tabids?.forEach((tabid) => {
        WOS.reloadWaveObject<Tab>(WOS.makeORef("tab", tabid));
    });
    ws.pinnedtabids?.forEach((tabid) => {
        WOS.reloadWaveObject<Tab>(WOS.makeORef("tab", tabid));
    });
}

function loadAllWorkspaceTabs(ws: Workspace) {
    if (ws == null || (!ws.tabids?.length && !ws.pinnedtabids?.length)) {
        return;
    }
    ws.tabids?.forEach((tabid) => {
        WOS.getObjectValue<Tab>(WOS.makeORef("tab", tabid));
    });
    ws.pinnedtabids?.forEach((tabid) => {
        WOS.getObjectValue<Tab>(WOS.makeORef("tab", tabid));
    });
}

async function initWave(initOpts: AgentMuxInitOpts) {
    getApi().sendLog("Init Wave " + JSON.stringify(initOpts));
    console.log(
        "Wave Init",
        "tabid",
        initOpts.tabId,
        "clientid",
        initOpts.clientId,
        "windowid",
        initOpts.windowId,
        "platform",
        platform
    );
    initGlobal({
        tabId: initOpts.tabId,
        clientId: initOpts.clientId,
        windowId: initOpts.windowId,
        platform,
        environment: "renderer",
        primaryTabStartup: initOpts.primaryTabStartup,
    });
    (window as any).globalAtoms = atoms;

    // Init WPS event handlers
    const globalWS = initWshrpc(initOpts.tabId);
    (window as any).globalWS = globalWS;
    (window as any).TabRpcClient = TabRpcClient;
    await loadConnStatus();
    initGlobalEventSubs(initOpts);
    subscribeToConnEvents();

    // ensures client/window/workspace are loaded into the cache before rendering
    const [client, waveWindow, initialTab] = await Promise.all([
        WOS.loadAndPinWaveObject<Client>(WOS.makeORef("client", initOpts.clientId)),
        WOS.loadAndPinWaveObject<WaveWindow>(WOS.makeORef("window", initOpts.windowId)),
        WOS.loadAndPinWaveObject<Tab>(WOS.makeORef("tab", initOpts.tabId)),
    ]);
    const [ws, layoutState] = await Promise.all([
        WOS.loadAndPinWaveObject<Workspace>(WOS.makeORef("workspace", waveWindow.workspaceid)),
        WOS.reloadWaveObject<LayoutState>(WOS.makeORef("layout", initialTab.layoutstate)),
    ]);
    loadAllWorkspaceTabs(ws);
    WOS.wpsSubscribeToObject(WOS.makeORef("workspace", waveWindow.workspaceid));

    document.title = `AgentMux ${appVersion} - ${initialTab.name}`; // TODO update with tab name change

    registerGlobalKeys();
    registerElectronReinjectKeyHandler();
    registerControlShiftStateUpdateHandler();
    await loadMonaco();
    const fullConfig = await RpcApi.GetFullConfigCommand(TabRpcClient);
    console.log("fullconfig", fullConfig);
    globalStore.set(atoms.fullConfigAtom, fullConfig);
    console.log("Wave First Render");
    let firstRenderResolveFn: () => void = null;
    let firstRenderPromise = new Promise<void>((resolve) => {
        firstRenderResolveFn = resolve;
    });
    const reactElem = createElement(App, { onFirstRender: firstRenderResolveFn }, null);
    const elem = document.getElementById("main");
    const root = createRoot(elem);
    root.render(reactElem);
    await firstRenderPromise;
    console.log("Wave First Render Done");

    // Hide startup loading message
    const startupLoading = document.getElementById("startup-loading");
    if (startupLoading) {
        startupLoading.remove();
    }

    getApi().setWindowInitStatus("wave-ready");
}
