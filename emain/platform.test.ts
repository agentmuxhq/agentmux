// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";

/**
 * Tests for portable multi-instance data directory logic
 *
 * These tests verify the "wave-data", "wave-data-2", "wave-data-3" logic:
 * - First instance uses wave-data/
 * - Second instance (if first locked) uses wave-data-2/
 * - Directories are cloned from wave-data/ if they don't exist
 * - Directories persist (never deleted)
 * - Lock detection works correctly
 */

describe("Portable Multi-Instance Logic", () => {
    let testDir: string;

    beforeEach(() => {
        // Create temporary test directory
        testDir = fs.mkdtempSync(path.join(os.tmpdir(), "wave-test-"));
    });

    afterEach(() => {
        // Clean up test directory
        if (fs.existsSync(testDir)) {
            fs.rmSync(testDir, { recursive: true, force: true });
        }
    });

    /**
     * Helper to simulate the isDirectoryLocked logic
     */
    function isDirectoryLocked(dirPath: string): boolean {
        const lockFile = path.join(dirPath, "wave.lock");
        if (!fs.existsSync(lockFile)) {
            return false;
        }

        try {
            const lockContent = fs.readFileSync(lockFile, "utf-8");
            const lockData = JSON.parse(lockContent);
            const pid = lockData.pid;

            try {
                process.kill(pid, 0);
                return true; // Process exists
            } catch (e) {
                return false; // Process doesn't exist, stale lock
            }
        } catch (e) {
            return false; // Can't read lock
        }
    }

    /**
     * Helper to create a lock file for testing
     */
    function createLockFile(dirPath: string, pid: number) {
        if (!fs.existsSync(dirPath)) {
            fs.mkdirSync(dirPath, { recursive: true });
        }
        const lockFile = path.join(dirPath, "wave.lock");
        fs.writeFileSync(lockFile, JSON.stringify({ pid, timestamp: Date.now() }));
    }

    /**
     * Helper to simulate findAvailableDataDirectory
     */
    function findAvailableDataDirectory(exeDir: string): string {
        const primaryDataDir = path.join(exeDir, "wave-data");

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
            }
            return primaryDataDir;
        }

        // Primary is locked, try numbered instances
        for (let i = 2; i <= 100; i++) {
            const dataDir = path.join(exeDir, `wave-data-${i}`);

            if (!isDirectoryLocked(dataDir)) {
                if (!fs.existsSync(dataDir)) {
                    // Clone from primary
                    copyDirectorySync(primaryDataDir, dataDir);
                }
                return dataDir;
            }
        }

        throw new Error("Too many Wave instances running (max 100)");
    }

    it("should use wave-data for first instance", () => {
        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(path.join(testDir, "wave-data"));
        expect(fs.existsSync(dataDir)).toBe(true);
    });

    it("should use wave-data-2 when wave-data is locked", () => {
        // Create primary and lock it
        createLockFile(path.join(testDir, "wave-data"), process.pid);

        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(path.join(testDir, "wave-data-2"));
        expect(fs.existsSync(dataDir)).toBe(true);
    });

    it("should use wave-data-3 when wave-data and wave-data-2 are locked", () => {
        // Create and lock primary
        createLockFile(path.join(testDir, "wave-data"), process.pid);
        // Create and lock secondary
        createLockFile(path.join(testDir, "wave-data-2"), process.pid);

        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(path.join(testDir, "wave-data-3"));
        expect(fs.existsSync(dataDir)).toBe(true);
    });

    it("should reuse existing wave-data-2 if not locked", () => {
        // Create primary
        const primary = path.join(testDir, "wave-data");
        fs.mkdirSync(primary, { recursive: true });
        fs.writeFileSync(path.join(primary, "test.txt"), "primary data");

        // Create wave-data-2 with existing data
        const secondary = path.join(testDir, "wave-data-2");
        fs.mkdirSync(secondary, { recursive: true });
        fs.writeFileSync(path.join(secondary, "settings.json"), '{"theme": "dark"}');

        // Lock primary
        createLockFile(primary, process.pid);

        // Should reuse wave-data-2
        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(secondary);

        // Settings should still exist (not overwritten)
        expect(fs.existsSync(path.join(secondary, "settings.json"))).toBe(true);
        const settings = fs.readFileSync(path.join(secondary, "settings.json"), "utf-8");
        expect(settings).toBe('{"theme": "dark"}');
    });

    it("should clone from primary when creating new instance", () => {
        // Create primary with some data
        const primary = path.join(testDir, "wave-data");
        fs.mkdirSync(primary, { recursive: true });
        fs.writeFileSync(path.join(primary, "config.json"), '{"version": "1.0"}');
        fs.mkdirSync(path.join(primary, "subdir"), { recursive: true });
        fs.writeFileSync(path.join(primary, "subdir", "nested.txt"), "nested content");

        // Lock primary
        createLockFile(primary, process.pid);

        // Create second instance
        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(path.join(testDir, "wave-data-2"));

        // Verify data was cloned
        expect(fs.existsSync(path.join(dataDir, "config.json"))).toBe(true);
        expect(fs.existsSync(path.join(dataDir, "subdir", "nested.txt"))).toBe(true);

        const config = fs.readFileSync(path.join(dataDir, "config.json"), "utf-8");
        expect(config).toBe('{"version": "1.0"}');
    });

    it("should handle stale lock files (process no longer running)", () => {
        // Create primary with a stale lock (non-existent PID)
        const primary = path.join(testDir, "wave-data");
        createLockFile(primary, 999999); // Very unlikely to exist

        // Should still use primary since lock is stale
        const dataDir = findAvailableDataDirectory(testDir);
        expect(dataDir).toBe(primary);
    });

    it("should throw error when all 100 slots are locked", () => {
        // Lock primary + 99 numbered instances
        createLockFile(path.join(testDir, "wave-data"), process.pid);
        for (let i = 2; i <= 100; i++) {
            createLockFile(path.join(testDir, `wave-data-${i}`), process.pid);
        }

        expect(() => findAvailableDataDirectory(testDir)).toThrow("Too many Wave instances running");
    });

    it("should correctly detect locked vs unlocked directories", () => {
        const lockedDir = path.join(testDir, "locked");
        const unlockedDir = path.join(testDir, "unlocked");
        const staleDir = path.join(testDir, "stale");

        // Create locked directory with current process lock
        createLockFile(lockedDir, process.pid);
        expect(isDirectoryLocked(lockedDir)).toBe(true);

        // Create unlocked directory (no lock file)
        fs.mkdirSync(unlockedDir, { recursive: true });
        expect(isDirectoryLocked(unlockedDir)).toBe(false);

        // Create stale lock
        createLockFile(staleDir, 999999);
        expect(isDirectoryLocked(staleDir)).toBe(false);
    });

    it("should use portable directory next to executable", () => {
        // This test verifies the main behavior:
        // Data should be stored next to the executable, not in AppData
        const result = findAvailableDataDirectory(testDir);

        // Should be inside testDir (next to "executable")
        expect(result.startsWith(testDir)).toBe(true);

        // Should be wave-data or wave-data-N
        const basename = path.basename(result);
        expect(basename).toMatch(/^wave-data(-\d+)?$/);
    });

    it("should preserve settings across restarts", () => {
        // Simulate: launch -> create settings -> close -> launch again

        // First launch
        const firstDataDir = findAvailableDataDirectory(testDir);
        expect(firstDataDir).toBe(path.join(testDir, "wave-data"));

        // Create some "settings"
        fs.writeFileSync(path.join(firstDataDir, "settings.json"), '{"theme":"dark"}');

        // Lock it (simulating running instance)
        createLockFile(firstDataDir, process.pid);

        // Second launch (while first is running)
        const secondDataDir = findAvailableDataDirectory(testDir);
        expect(secondDataDir).toBe(path.join(testDir, "wave-data-2"));

        // Now "close" first instance by removing lock
        fs.unlinkSync(path.join(firstDataDir, "wave.lock"));

        // Third launch (after closing first)
        const thirdDataDir = findAvailableDataDirectory(testDir);
        expect(thirdDataDir).toBe(firstDataDir); // Reuses original

        // Settings should still exist!
        const settings = fs.readFileSync(path.join(thirdDataDir, "settings.json"), "utf-8");
        expect(settings).toBe('{"theme":"dark"}');
    });
});
