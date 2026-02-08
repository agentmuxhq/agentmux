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
    initGlobalWaveEventSubs,
    loadConnStatus,
    pushFlashError,
    pushNotification,
    removeNotificationById,
    subscribeToConnEvents,
} from "@/store/global";
import * as WOS from "@/store/wos";
import { loadFonts } from "@/util/fontutil";
import { setKeyUtilPlatform } from "@/util/keyutil";
import { createElement } from "react";
import { createRoot } from "react-dom/client";

console.log("[wave.ts] Module loading...");
console.log("[wave.ts] window.api available:", !!(window as any).api);

const platform = getApi().getPlatform();
console.log("[wave.ts] Got platform:", platform);

const appVersion = getApi().getAboutModalDetails().version;
console.log("[wave.ts] Got app version:", appVersion);

document.title = `Wave Terminal ${appVersion}`;
let savedInitOpts: WaveInitOpts = null;

console.log("[wave.ts] Module initialization complete");

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
    console.log("update zoomfactor", zoomFactor);
    document.documentElement.style.setProperty("--zoomfactor", String(zoomFactor));
    document.documentElement.style.setProperty("--zoomfactor-inv", String(1 / zoomFactor));
}

async function initBare() {
    console.log("[initBare] Starting...");
    getApi().sendLog("Init Bare");
    console.log("[initBare] Hiding body...");
    document.body.style.visibility = "hidden";
    document.body.style.opacity = "0";
    document.body.classList.add("is-transparent");
    console.log("[initBare] Registering onWaveInit...");
    getApi().onWaveInit(initWaveWrap);
    console.log("[initBare] Setting platform...");
    setKeyUtilPlatform(platform);
    console.log("[initBare] Loading fonts...");
    loadFonts();
    console.log("[initBare] Setting zoom...");
    updateZoomFactor(getApi().getZoomFactor());
    getApi().onZoomFactorChange((zoomFactor) => {
        updateZoomFactor(zoomFactor);
    });
    console.log("[initBare] Waiting for fonts...");
    document.fonts.ready.then(() => {
        console.log("[initBare] Fonts ready!");
        getApi().sendLog("Init Bare Done");
        console.log("[initBare] Setting window status to ready...");
        getApi().setWindowInitStatus("ready");

        // TEMPORARY: Manually trigger wave-init for Tauri testing
        // TODO: Implement proper Tauri window state management (Phase 8)
        if (typeof (window as any).__TAURI_INTERNALS__ !== "undefined") {
            console.log("[initBare] TAURI: Manually triggering wave-init with dummy IDs");
            setTimeout(() => {
                const dummyInitOpts: WaveInitOpts = {
                    tabId: "dummy-tab-001",
                    clientId: "dummy-client-001",
                    windowId: "dummy-window-001",
                    activate: true,
                    primaryTabStartup: true
                };
                initWaveWrap(dummyInitOpts);
            }, 100);
        }

        console.log("[initBare] Complete!");
    });
    console.log("[initBare] Setup complete, fonts loading async...");
}

// Handle both cases: DOM not yet loaded, or already loaded (Tauri dynamic import)
if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initBare);
} else {
    // DOM already loaded (e.g., when dynamically imported from tauri-bootstrap)
    initBare();
}

async function initWaveWrap(initOpts: WaveInitOpts) {
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
    document.title = `Wave Terminal ${appVersion} - ${initialTab.name}`; // TODO update with tab name change
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

async function initWave(initOpts: WaveInitOpts) {
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
    initGlobalWaveEventSubs(initOpts);
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

    document.title = `Wave Terminal ${appVersion} - ${initialTab.name}`; // TODO update with tab name change

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
    getApi().setWindowInitStatus("wave-ready");
}
