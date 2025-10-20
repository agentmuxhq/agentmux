// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { fireAndForget } from "@/util/util";
import { app, dialog, ipcMain, shell } from "electron";
import envPaths from "env-paths";
import { existsSync, mkdirSync } from "fs";
import os from "os";
import path from "path";
import { WaveDevVarName, WaveDevViteVarName } from "../frontend/util/isdev";
import * as keyutil from "../frontend/util/keyutil";
import packageJson from "../package.json";

// This is a little trick to ensure that Electron puts all its runtime data into a subdirectory to avoid conflicts with our own data.
// On macOS, it will store to ~/Library/Application \Support/waveterm/electron
// On Linux, it will store to ~/.config/waveterm/electron
// On Windows, it will store to %LOCALAPPDATA%/waveterm/electron
app.setName("waveterm/electron");

const isDev = !app.isPackaged;
const isDevVite = isDev && process.env.ELECTRON_RENDERER_URL;
console.log(`Running in ${isDev ? "development" : "production"} mode`);
if (isDev) {
    process.env[WaveDevVarName] = "1";
}
if (isDevVite) {
    process.env[WaveDevViteVarName] = "1";
}

/**
 * Find an available data directory for this instance (portable mode).
 *
 * Searches for unlocked data directories next to the executable:
 * - wave-data/ (primary)
 * - wave-data-2/, wave-data-3/, etc. (additional instances)
 *
 * If a directory doesn't exist, it's created by copying from wave-data.
 * Directories are never deleted, allowing settings to persist across runs.
 *
 * Example flow:
 * - Launch 1: Uses wave-data/
 * - Launch 2 (while 1 running): Copies to wave-data-2/ and uses it
 * - Launch 3 (while 1&2 running): Copies to wave-data-3/ and uses it
 * - Close all → Launch 1: Uses wave-data/ (settings preserved)
 * - Launch 2: Uses wave-data-2/ (settings from last run preserved)
 *
 * @returns Path to the data directory to use
 */
function findAvailableDataDirectory(): string {
    const fs = require("fs");
    const exeDir = path.dirname(app.getPath("exe"));
    const primaryDataDir = path.join(exeDir, "wave-data");

    // Helper to check if a directory is locked
    function isDirectoryLocked(dirPath: string): boolean {
        const lockFile = path.join(dirPath, "wave.lock");
        if (!fs.existsSync(lockFile)) {
            return false;
        }

        try {
            // Try to read the lock file
            const lockContent = fs.readFileSync(lockFile, "utf-8");
            const lockData = JSON.parse(lockContent);
            const pid = lockData.pid;

            // Check if the process is still running
            try {
                process.kill(pid, 0); // Signal 0 checks if process exists without killing
                return true; // Process exists, directory is locked
            } catch (e) {
                // Process doesn't exist, lock is stale
                return false;
            }
        } catch (e) {
            // Can't read lock file, assume not locked
            return false;
        }
    }

    // Helper to copy directory recursively
    function copyDirectorySync(src: string, dest: string) {
        if (!fs.existsSync(dest)) {
            fs.mkdirSync(dest, { recursive: true });
        }

        const entries = fs.readdirSync(src, { withFileTypes: true });

        for (const entry of entries) {
            const srcPath = path.join(src, entry.name);
            const destPath = path.join(dest, entry.name);

            if (entry.isDirectory()) {
                copyDirectorySync(srcPath, destPath);
            } else {
                fs.copyFileSync(srcPath, destPath);
            }
        }
    }

    // Try primary directory first
    if (!isDirectoryLocked(primaryDataDir)) {
        if (!fs.existsSync(primaryDataDir)) {
            fs.mkdirSync(primaryDataDir, { recursive: true });
            console.log(`Created primary data directory: ${primaryDataDir}`);
        }
        return primaryDataDir;
    }

    // Primary is locked, try numbered instances
    for (let i = 2; i <= 100; i++) {
        const dataDir = path.join(exeDir, `wave-data-${i}`);

        if (!isDirectoryLocked(dataDir)) {
            if (!fs.existsSync(dataDir)) {
                // Clone from primary
                console.log(`Cloning ${primaryDataDir} → ${dataDir}`);
                copyDirectorySync(primaryDataDir, dataDir);
            }
            return dataDir;
        }
    }

    // All 100 slots are locked
    throw new Error("Too many Wave instances running (max 100)");
}

// For backward compatibility with old paths (legacy/environment variable overrides)
const waveDirName = isDev ? "waveterm-dev" : "waveterm";
const waveConfigDirName = waveDirName; // Config uses same directory name

// Find available data directory (portable mode - next to executable)
const dataDirectory = findAvailableDataDirectory();
const baseNameParts = path.basename(dataDirectory).split("-");
const instanceNumber = baseNameParts.length > 2 ? baseNameParts[2] : "1";

// Set app name to include instance number
const version = packageJson.version;
let appName = isDev ? `Wave ${version} (Dev)` : `Wave ${version}`;
if (instanceNumber !== "1") {
    appName = `${appName} [Instance ${instanceNumber}]`;
}
app.setName(appName);

// Override envPaths to use our portable directory
const paths = {
    data: dataDirectory,
    config: path.join(dataDirectory, "config"),
    cache: path.join(dataDirectory, "cache"),
    log: path.join(dataDirectory, "logs"),
    temp: path.join(dataDirectory, "temp"),
};
const unamePlatform = process.platform;
const unameArch: string = process.arch;
keyutil.setKeyUtilPlatform(unamePlatform);

const WaveConfigHomeVarName = "WAVETERM_CONFIG_HOME";
const WaveDataHomeVarName = "WAVETERM_DATA_HOME";
const WaveHomeVarName = "WAVETERM_HOME";

export function checkIfRunningUnderARM64Translation(fullConfig: FullConfigType) {
    if (!fullConfig.settings["app:dismissarchitecturewarning"] && app.runningUnderARM64Translation) {
        console.log("Running under ARM64 translation, alerting user");
        const dialogOpts: Electron.MessageBoxOptions = {
            type: "warning",
            buttons: ["Dismiss", "Learn More"],
            title: "Wave has detected a performance issue",
            message: `Wave is running in ARM64 translation mode which may impact performance.\n\nRecommendation: Download the native ARM64 version from our website for optimal performance.`,
        };

        const choice = dialog.showMessageBoxSync(null, dialogOpts);
        if (choice === 1) {
            // Open the documentation URL
            console.log("User chose to learn more");
            fireAndForget(() =>
                shell.openExternal(
                    "https://docs.waveterm.dev/faq#why-does-wave-warn-me-about-arm64-translation-when-it-launches"
                )
            );
            throw new Error("User redirected to docsite to learn more about ARM64 translation, exiting");
        } else {
            console.log("User dismissed the dialog");
        }
    }
}

/**
 * Gets the path to the old Wave home directory (defaults to `~/.waveterm`).
 * @returns The path to the directory if it exists and contains valid data for the current app, otherwise null.
 */
function getWaveHomeDir(): string {
    let home = process.env[WaveHomeVarName];
    if (!home) {
        const homeDir = app.getPath("home");
        if (homeDir) {
            home = path.join(homeDir, `.${waveDirName}`);
        }
    }
    // If home exists and it has `wave.lock` in it, we know it has valid data from Wave >=v0.8. Otherwise, it could be for WaveLegacy (<v0.8)
    if (home && existsSync(home) && existsSync(path.join(home, "wave.lock"))) {
        return home;
    }
    return null;
}

/**
 * Ensure the given path exists, creating it recursively if it doesn't.
 * @param path The path to ensure.
 * @returns The same path, for chaining.
 */
function ensurePathExists(path: string): string {
    if (!existsSync(path)) {
        mkdirSync(path, { recursive: true });
    }
    return path;
}

/**
 * Gets the path to the directory where Wave configurations are stored. Creates the directory if it does not exist.
 * Handles backwards compatibility with the old Wave Home directory model, where configurations and data were stored together.
 * @returns The path where configurations should be stored.
 */
function getWaveConfigDir(): string {
    // If wave home dir exists, use it for backwards compatibility
    const waveHomeDir = getWaveHomeDir();
    if (waveHomeDir) {
        return path.join(waveHomeDir, "config");
    }

    const override = process.env[WaveConfigHomeVarName];
    const xdgConfigHome = process.env.XDG_CONFIG_HOME;
    let retVal: string;
    if (override) {
        retVal = override;
    } else if (xdgConfigHome) {
        retVal = path.join(xdgConfigHome, waveConfigDirName);
    } else {
        retVal = path.join(app.getPath("home"), ".config", waveConfigDirName);
    }
    return ensurePathExists(retVal);
}

/**
 * Gets the path to the directory where Wave data is stored. Creates the directory if it does not exist.
 * Handles backwards compatibility with the old Wave Home directory model, where configurations and data were stored together.
 * @returns The path where data should be stored.
 */
function getWaveDataDir(): string {
    // If wave home dir exists, use it for backwards compatibility
    const waveHomeDir = getWaveHomeDir();
    if (waveHomeDir) {
        return waveHomeDir;
    }

    const override = process.env[WaveDataHomeVarName];
    const xdgDataHome = process.env.XDG_DATA_HOME;
    let retVal: string;
    if (override) {
        retVal = override;
    } else if (xdgDataHome) {
        retVal = path.join(xdgDataHome, waveDirName);
    } else {
        retVal = paths.data;
    }
    return ensurePathExists(retVal);
}

function getElectronAppBasePath(): string {
    return path.dirname(import.meta.dirname);
}

function getElectronAppUnpackedBasePath(): string {
    return getElectronAppBasePath().replace("app.asar", "app.asar.unpacked");
}

const wavesrvBinName = `wavesrv.${unameArch}`;

function getWaveSrvPath(): string {
    if (process.platform === "win32") {
        const winBinName = `${wavesrvBinName}.exe`;
        const appPath = path.join(getElectronAppUnpackedBasePath(), "bin", winBinName);
        return `${appPath}`;
    }
    return path.join(getElectronAppUnpackedBasePath(), "bin", wavesrvBinName);
}

function getWaveSrvCwd(): string {
    return getWaveDataDir();
}

ipcMain.on("get-is-dev", (event) => {
    event.returnValue = isDev;
});
ipcMain.on("get-platform", (event, url) => {
    event.returnValue = unamePlatform;
});
ipcMain.on("get-user-name", (event) => {
    const userInfo = os.userInfo();
    event.returnValue = userInfo.username;
});
ipcMain.on("get-host-name", (event) => {
    event.returnValue = os.hostname();
});
ipcMain.on("get-webview-preload", (event) => {
    event.returnValue = path.join(getElectronAppBasePath(), "preload", "preload-webview.cjs");
});
ipcMain.on("get-data-dir", (event) => {
    event.returnValue = getWaveDataDir();
});
ipcMain.on("get-config-dir", (event) => {
    event.returnValue = getWaveConfigDir();
});

/**
 * Gets the value of the XDG_CURRENT_DESKTOP environment variable. If ORIGINAL_XDG_CURRENT_DESKTOP is set, it will be returned instead.
 * This corrects for a strange behavior in Electron, where it sets its own value for XDG_CURRENT_DESKTOP to improve Chromium compatibility.
 * @see https://www.electronjs.org/docs/latest/api/environment-variables#original_xdg_current_desktop
 * @returns The value of the XDG_CURRENT_DESKTOP environment variable, or ORIGINAL_XDG_CURRENT_DESKTOP if set, or undefined if neither are set.
 */
function getXdgCurrentDesktop(): string {
    if (process.env.ORIGINAL_XDG_CURRENT_DESKTOP) {
        return process.env.ORIGINAL_XDG_CURRENT_DESKTOP;
    } else if (process.env.XDG_CURRENT_DESKTOP) {
        return process.env.XDG_CURRENT_DESKTOP;
    } else {
        return undefined;
    }
}

/**
 * Calls the given callback with the value of the XDG_CURRENT_DESKTOP environment variable set to ORIGINAL_XDG_CURRENT_DESKTOP if it is set.
 * @see https://www.electronjs.org/docs/latest/api/environment-variables#original_xdg_current_desktop
 * @param callback The callback to call.
 */
function callWithOriginalXdgCurrentDesktop(callback: () => void) {
    const currXdgCurrentDesktopDefined = "XDG_CURRENT_DESKTOP" in process.env;
    const currXdgCurrentDesktop = process.env.XDG_CURRENT_DESKTOP;
    const originalXdgCurrentDesktop = getXdgCurrentDesktop();
    if (originalXdgCurrentDesktop) {
        process.env.XDG_CURRENT_DESKTOP = originalXdgCurrentDesktop;
    }
    callback();
    if (originalXdgCurrentDesktop) {
        if (currXdgCurrentDesktopDefined) {
            process.env.XDG_CURRENT_DESKTOP = currXdgCurrentDesktop;
        } else {
            delete process.env.XDG_CURRENT_DESKTOP;
        }
    }
}

/**
 * Calls the given async callback with the value of the XDG_CURRENT_DESKTOP environment variable set to ORIGINAL_XDG_CURRENT_DESKTOP if it is set.
 * @see https://www.electronjs.org/docs/latest/api/environment-variables#original_xdg_current_desktop
 * @param callback The async callback to call.
 */
async function callWithOriginalXdgCurrentDesktopAsync(callback: () => Promise<void>) {
    const currXdgCurrentDesktopDefined = "XDG_CURRENT_DESKTOP" in process.env;
    const currXdgCurrentDesktop = process.env.XDG_CURRENT_DESKTOP;
    const originalXdgCurrentDesktop = getXdgCurrentDesktop();
    if (originalXdgCurrentDesktop) {
        process.env.XDG_CURRENT_DESKTOP = originalXdgCurrentDesktop;
    }
    await callback();
    if (originalXdgCurrentDesktop) {
        if (currXdgCurrentDesktopDefined) {
            process.env.XDG_CURRENT_DESKTOP = currXdgCurrentDesktop;
        } else {
            delete process.env.XDG_CURRENT_DESKTOP;
        }
    }
}

/**
 * Gets multi-instance information.
 * @returns Object containing isMultiInstance flag and instanceId
 */
function getMultiInstanceInfo(): { isMultiInstance: boolean; instanceId: string | null } {
    const isMultiInstance = instanceNumber !== "1";
    const instanceId = isMultiInstance ? `instance-${instanceNumber}` : null;
    return { isMultiInstance, instanceId };
}

export {
    callWithOriginalXdgCurrentDesktop,
    callWithOriginalXdgCurrentDesktopAsync,
    getElectronAppBasePath,
    getElectronAppUnpackedBasePath,
    getMultiInstanceInfo,
    getWaveConfigDir,
    getWaveDataDir,
    getWaveSrvCwd,
    getWaveSrvPath,
    getXdgCurrentDesktop,
    isDev,
    isDevVite,
    unameArch,
    unamePlatform,
    WaveConfigHomeVarName,
    WaveDataHomeVarName,
};
