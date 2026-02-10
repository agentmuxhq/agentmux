// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

// Typed React hooks for accessing Wave domain objects and services.
// These hooks encapsulate Jotai atom access and WOS patterns, providing
// a clean, typed API for view components.

import { useAtomValue } from "jotai";
import { atoms, getConnStatusAtom, getSettingsKeyAtom, globalStore } from "@/app/store/global";
import * as WOS from "@/app/store/wos";

/**
 * Get the current workspace (derived from the window's workspaceid).
 * Returns null during initialization.
 */
export function useWorkspace(): Workspace | null {
    return useAtomValue(atoms.workspace);
}

/**
 * Get the current tab for this tab view.
 * Returns null during initialization.
 */
export function useTab(): Tab | null {
    return useAtomValue(atoms.tabAtom);
}

/**
 * Get the current client data.
 * Returns null during initialization.
 */
export function useClient(): Client | null {
    return useAtomValue(atoms.client);
}

/**
 * Get the current window data.
 * Returns null during initialization.
 */
export function useWaveWindow(): WaveWindow | null {
    return useAtomValue(atoms.waveWindow);
}

/**
 * Get the full application config (settings + connections + presets).
 * Returns null before config is loaded.
 */
export function useFullConfig(): FullConfigType | null {
    return useAtomValue(atoms.fullConfigAtom);
}

/**
 * Get the application settings.
 * Returns an empty object before config is loaded.
 */
export function useSettings(): SettingsType {
    return useAtomValue(atoms.settingsAtom);
}

/**
 * Get a single settings value by key.
 */
export function useSettingsKey<T extends keyof SettingsType>(key: T): SettingsType[T] {
    return useAtomValue(getSettingsKeyAtom(key));
}

/**
 * Get the connection status for a named connection.
 * Returns a reactive ConnStatus that updates on connection state changes.
 */
export function useConnStatus(connName: string): ConnStatus {
    const statusAtom = getConnStatusAtom(connName);
    return useAtomValue(statusAtom);
}

/**
 * Get all connection statuses as an array.
 */
export function useAllConnStatuses(): ConnStatus[] {
    return useAtomValue(atoms.allConnStatus);
}

/**
 * Get a typed WaveObj by ORef string (e.g., "block:abc-123").
 * Returns null if the object isn't loaded yet.
 */
export function useWaveObject<T extends WaveObj>(oref: string): T | null {
    const objAtom = WOS.getWaveObjectAtom<T>(oref);
    return useAtomValue(objAtom);
}

/**
 * Check whether the updater has a pending update.
 */
export function useUpdaterStatus(): UpdaterStatus {
    return useAtomValue(atoms.updaterStatusAtom);
}

/**
 * Get the current flash errors list.
 */
export function useFlashErrors(): FlashErrorType[] {
    return useAtomValue(atoms.flashErrors);
}

/**
 * Get the current notifications list.
 */
export function useNotifications(): NotificationType[] {
    return useAtomValue(atoms.notifications);
}

/**
 * Whether the window is in full-screen mode.
 */
export function useIsFullScreen(): boolean {
    return useAtomValue(atoms.isFullScreen);
}

/**
 * Whether the user prefers reduced motion (system or user setting).
 */
export function usePrefersReducedMotion(): boolean {
    return useAtomValue(atoms.prefersReducedMotionAtom);
}
