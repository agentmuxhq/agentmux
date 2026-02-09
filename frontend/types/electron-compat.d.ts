// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Compatibility types for Electron APIs (post-Tauri migration)
// These types replace Electron.* namespace types that are no longer available

declare namespace Electron {
    interface Point {
        x: number;
        y: number;
    }

    interface Rectangle {
        x: number;
        y: number;
        width: number;
        height: number;
    }
}
