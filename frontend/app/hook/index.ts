// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

// Re-export all hooks from this directory for convenient imports:
//   import { useWorkspace, useTab, useSettings } from "@/app/hook";

export { useDimensionsWithCallbackRef, useDimensionsWithExistingRef } from "./useDimensions";
export { useLongClick } from "./useLongClick";
export {
    useWorkspace,
    useTab,
    useClient,
    useWaveWindow,
    useFullConfig,
    useSettings,
    useSettingsKey,
    useConnStatus,
    useAllConnStatuses,
    useWaveObject,
    useUpdaterStatus,
    useFlashErrors,
    useNotifications,
    useIsFullScreen,
    usePrefersReducedMotion,
} from "./useWaveState";
export { useNotificationActions, useFlashErrorActions } from "./useNotifications";
