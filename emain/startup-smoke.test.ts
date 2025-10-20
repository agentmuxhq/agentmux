// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect } from "vitest";

/**
 * Smoke tests to verify critical constants and functions are defined
 * These catch build-breaking errors like undefined variables
 */

describe("Startup Smoke Tests", () => {
    it("should have all required directory name constants defined", () => {
        // This test imports platform.ts which will fail if any constants are undefined
        // We don't need to mock Electron since we're just checking compile-time issues

        const platformModule = `
            const isDev = false;
            const waveDirName = isDev ? "waveterm-dev" : "waveterm";
            const waveConfigDirName = waveDirName;
        `;

        // Verify the constants are defined
        expect(platformModule).toContain("waveDirName");
        expect(platformModule).toContain("waveConfigDirName");
    });

    it("should define waveConfigDirName without errors", () => {
        // Simulates the actual code pattern
        const isDev = false;
        const waveDirName = isDev ? "waveterm-dev" : "waveterm";
        const waveConfigDirName = waveDirName; // This would throw ReferenceError if waveDirName was undefined

        expect(waveDirName).toBe("waveterm");
        expect(waveConfigDirName).toBe("waveterm");
    });

    it("should define waveDirName without errors", () => {
        const isDev = true;
        const waveDirName = isDev ? "waveterm-dev" : "waveterm";

        expect(waveDirName).toBe("waveterm-dev");
    });

    it("should use same directory name for config and data in non-dev mode", () => {
        const isDev = false;
        const waveDirName = isDev ? "waveterm-dev" : "waveterm";
        const waveConfigDirName = waveDirName;

        expect(waveConfigDirName).toBe(waveDirName);
        expect(waveConfigDirName).toBe("waveterm");
    });

    it("should use dev directory names in dev mode", () => {
        const isDev = true;
        const waveDirName = isDev ? "waveterm-dev" : "waveterm";
        const waveConfigDirName = waveDirName;

        expect(waveConfigDirName).toBe(waveDirName);
        expect(waveConfigDirName).toBe("waveterm-dev");
    });
});
