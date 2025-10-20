// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as electron from "electron";
import * as child_process from "node:child_process";
import * as readline from "readline";
import { WebServerEndpointVarName, WSServerEndpointVarName } from "../frontend/util/endpoints";
import { AuthKey, WaveAuthKeyEnv } from "./authkey";
import { setForceQuit } from "./emain-activity";
import { WaveAppPathVarName, WaveAppElectronExecPath, getElectronExecPath } from "./emain-util";
import {
    getElectronAppUnpackedBasePath,
    getWaveConfigDir,
    getWaveDataDir,
    getWaveSrvCwd,
    getWaveSrvPath,
    getXdgCurrentDesktop,
    WaveConfigHomeVarName,
    WaveDataHomeVarName,
} from "./platform";
import { updater } from "./updater";

let isWaveSrvDead = false;
let waveSrvProc: child_process.ChildProcessWithoutNullStreams | null = null;
let WaveVersion = "unknown"; // set by WAVESRV-ESTART
let WaveBuildTime = 0; // set by WAVESRV-ESTART
let waveSrvLockError = false; // set when wavesrv fails to acquire lock

export function getWaveVersion(): { version: string; buildTime: number } {
    return { version: WaveVersion, buildTime: WaveBuildTime };
}

let waveSrvReadyResolve = (value: boolean) => {};
const waveSrvReady: Promise<boolean> = new Promise((resolve, _) => {
    waveSrvReadyResolve = resolve;
});

export function getWaveSrvReady(): Promise<boolean> {
    return waveSrvReady;
}

export function getWaveSrvProc(): child_process.ChildProcessWithoutNullStreams | null {
    return waveSrvProc;
}

export function getIsWaveSrvDead(): boolean {
    return isWaveSrvDead;
}

export function getWaveSrvLockError(): boolean {
    return waveSrvLockError;
}

export async function showMultiInstanceDialog(): Promise<void> {
    try {
        await electron.app.whenReady();
        const { dialog, shell } = electron;
        const dialogOpts: Electron.MessageBoxOptions = {
            type: "info",
            buttons: ["Close", "Learn More"],
            defaultId: 0,
            cancelId: 0,
            title: "Wave is Already Running",
            message: "Another instance of Wave is already running.",
            detail:
                "Wave is already running on this system. To run multiple instances simultaneously, " +
                "launch Wave with the --instance flag:\n\n" +
                "Example:\n" +
                "  Wave.exe --instance=test\n" +
                "  Wave.exe --instance=dev\n\n" +
                "Each instance will have its own isolated data while sharing your settings.\n\n" +
                "Click 'Learn More' for documentation on multi-instance mode.",
            noLink: true,
        };

        const choice = dialog.showMessageBoxSync(dialogOpts);
        if (choice === 1) {
            // Learn More button
            await shell.openExternal("https://docs.waveterm.dev/");
        }
    } catch (e) {
        console.log("error showing multi-instance dialog:", e);
    }
}

export async function showSingleInstanceDialog(): Promise<void> {
    try {
        await electron.app.whenReady();
        const { dialog, shell } = electron;
        const dialogOpts: Electron.MessageBoxOptions = {
            type: "info",
            buttons: ["Close", "Learn More"],
            defaultId: 0,
            cancelId: 0,
            title: "Wave is Already Running in Single-Instance Mode",
            message: "Another instance of Wave is already running with --single-instance flag.",
            detail:
                "You launched Wave with the --single-instance flag, which prevents multiple instances.\n\n" +
                "To run multiple instances, simply launch Wave without the --single-instance flag:\n\n" +
                "  Wave.exe                     (auto multi-instance)\n" +
                "  Wave.exe --instance=test     (named multi-instance)\n\n" +
                "Each instance will have its own isolated data while sharing your settings.\n\n" +
                "Click 'Learn More' for documentation on multi-instance mode.",
            noLink: true,
        };

        const choice = dialog.showMessageBoxSync(dialogOpts);
        if (choice === 1) {
            // Learn More button
            await shell.openExternal("https://docs.waveterm.dev/");
        }
    } catch (e) {
        console.log("error showing single-instance dialog:", e);
    }
}

export function runWaveSrv(handleWSEvent: (evtMsg: WSEventType) => void): Promise<boolean> {
    let pResolve: (value: boolean) => void;
    let pReject: (reason?: any) => void;
    const rtnPromise = new Promise<boolean>((argResolve, argReject) => {
        pResolve = argResolve;
        pReject = argReject;
    });
    const envCopy = { ...process.env };
    const xdgCurrentDesktop = getXdgCurrentDesktop();
    if (xdgCurrentDesktop != null) {
        envCopy["XDG_CURRENT_DESKTOP"] = xdgCurrentDesktop;
    }
    envCopy[WaveAppPathVarName] = getElectronAppUnpackedBasePath();
    envCopy[WaveAppElectronExecPath] = getElectronExecPath();
    envCopy[WaveAuthKeyEnv] = AuthKey;
    envCopy[WaveDataHomeVarName] = getWaveDataDir();
    envCopy[WaveConfigHomeVarName] = getWaveConfigDir();
    // Set cloud endpoints for dev mode
    envCopy["WCLOUD_ENDPOINT"] = "https://api.waveterm.dev/central";
    envCopy["WCLOUD_WS_ENDPOINT"] = "wss://wsapi.waveterm.dev/";
    const waveSrvCmd = getWaveSrvPath();
    console.log("trying to run local server", waveSrvCmd);
    const proc = child_process.spawn(getWaveSrvPath(), {
        cwd: getWaveSrvCwd(),
        env: envCopy,
    });
    proc.on("exit", async (e) => {
        if (updater?.status == "installing") {
            return;
        }
        console.log("wavesrv exited, shutting down");

        // If wavesrv failed due to lock conflict, show multi-instance dialog
        if (waveSrvLockError) {
            await showMultiInstanceDialog();
        }

        setForceQuit(true);
        isWaveSrvDead = true;
        electron.app.quit();
    });
    proc.on("spawn", (e) => {
        console.log("spawned wavesrv");
        waveSrvProc = proc;
        pResolve(true);
    });
    proc.on("error", (e) => {
        console.log("error running wavesrv", e);
        pReject(e);
    });
    const rlStdout = readline.createInterface({
        input: proc.stdout,
        terminal: false,
    });
    rlStdout.on("line", (line) => {
        console.log(line);
    });
    const rlStderr = readline.createInterface({
        input: proc.stderr,
        terminal: false,
    });
    rlStderr.on("line", (line) => {
        if (line.includes("WAVESRV-ESTART")) {
            const startParams = /ws:([a-z0-9.:]+) web:([a-z0-9.:]+) version:([a-z0-9.\-]+) buildtime:(\d+)/gm.exec(
                line
            );
            if (startParams == null) {
                console.log("error parsing WAVESRV-ESTART line", line);
                electron.app.quit();
                return;
            }
            process.env[WSServerEndpointVarName] = startParams[1];
            process.env[WebServerEndpointVarName] = startParams[2];
            WaveVersion = startParams[3];
            WaveBuildTime = parseInt(startParams[4]);
            waveSrvReadyResolve(true);
            return;
        }
        if (line.startsWith("WAVESRV-EVENT:")) {
            const evtJson = line.slice("WAVESRV-EVENT:".length);
            try {
                const evtMsg: WSEventType = JSON.parse(evtJson);
                handleWSEvent(evtMsg);
            } catch (e) {
                console.log("error handling WAVESRV-EVENT", e);
            }
            return;
        }
        // Detect lock error from wavesrv
        if (line.includes("error acquiring wave lock") || line.includes("lock already acquired")) {
            console.log("wavesrv detected lock conflict:", line);
            waveSrvLockError = true;
        }
        console.log(line);
    });
    return rtnPromise;
}
