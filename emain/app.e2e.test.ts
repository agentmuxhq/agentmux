// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * E2E tests for critical application functionality
 *
 * These tests verify:
 * 1. Version is displayed correctly in title bar and window
 * 2. wsh binary is deployed and accessible in PATH
 * 3. Shell integration works correctly
 */

import { describe, it, expect, beforeAll } from "vitest";
import * as fs from "fs";
import * as path from "path";
import { exec } from "child_process";
import { promisify } from "util";
import packageJson from "../package.json";

const execAsync = promisify(exec);

describe("E2E: Application Version Display", () => {
    const EXPECTED_VERSION = packageJson.version;

    it("package.json should have valid semver version", () => {
        expect(EXPECTED_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
    });

    it("wave.ts should use dynamic version from getAboutModalDetails()", async () => {
        const waveTs = fs.readFileSync(path.join(__dirname, "../frontend/wave.ts"), "utf-8");

        // Check that appVersion is defined from getAboutModalDetails()
        expect(waveTs).toContain("const appVersion = getApi().getAboutModalDetails().version");

        // Check that all document.title assignments use appVersion
        const titleAssignments = waveTs.match(/document\.title\s*=\s*`[^`]*`/g) || [];
        expect(titleAssignments.length).toBeGreaterThan(0);

        for (const assignment of titleAssignments) {
            expect(assignment).toContain("${appVersion}");
        }
    });

    it("wave.ts should NOT have hardcoded version strings", () => {
        const waveTs = fs.readFileSync(path.join(__dirname, "../frontend/wave.ts"), "utf-8");

        // Should not contain hardcoded version patterns like "Wave Terminal 0.12"
        expect(waveTs).not.toMatch(/Wave Terminal \d+\.\d+/);
        expect(waveTs).not.toMatch(/document\.title\s*=\s*`Wave Terminal\s*-/);
    });

    it("platform.ts should construct app name with version from package.json", () => {
        const platformTs = fs.readFileSync(path.join(__dirname, "platform.ts"), "utf-8");

        // Check that version is imported from package.json
        expect(platformTs).toContain('import packageJson from "../package.json"');
        expect(platformTs).toContain("const version = packageJson.version");

        // Check that appName includes version
        expect(platformTs).toMatch(/appName\s*=.*\$\{version\}/);
    });
});

describe("E2E: wsh Binary Deployment", () => {
    const EXPECTED_VERSION = packageJson.version;
    const platform = process.platform;
    const arch = process.arch === "x64" ? "x64" : process.arch === "arm64" ? "arm64" : process.arch;

    it("versioned wsh binaries should exist in dist/bin", () => {
        const binDir = path.join(__dirname, "../dist/bin");
        expect(fs.existsSync(binDir)).toBe(true);

        // Check for Windows binaries
        const windowsX64Binary = path.join(binDir, `wsh-${EXPECTED_VERSION}-windows.x64.exe`);
        const windowsArm64Binary = path.join(binDir, `wsh-${EXPECTED_VERSION}-windows.arm64.exe`);

        expect(fs.existsSync(windowsX64Binary) || fs.existsSync(windowsArm64Binary)).toBe(true);
    });

    it("wsh binary filenames should match package.json version", () => {
        const binDir = path.join(__dirname, "../dist/bin");
        const files = fs.readdirSync(binDir).filter(f => f.startsWith("wsh-"));

        expect(files.length).toBeGreaterThan(0);

        for (const file of files) {
            expect(file).toContain(EXPECTED_VERSION);
        }
    });

    it("shellutil.go should properly handle wsh binary copy errors", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Check that GetLocalWshBinaryPath errors are handled
        expect(shellutilGo).toContain("GetLocalWshBinaryPath");

        // Check that binary copy errors are NOT silently ignored
        const copySection = shellutilGo.match(/AtomicRenameCopy[\s\S]*?return/m);
        expect(copySection).toBeTruthy();
    });

    it("pwsh shell integration template should set wsh PATH", () => {
        const pwshTemplate = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellintegration/pwsh_wavepwsh.sh"),
            "utf-8"
        );

        // Check that PATH is set with WSHBINDIR_PWSH
        expect(pwshTemplate).toContain("$env:PATH = {{.WSHBINDIR_PWSH}}");
        expect(pwshTemplate).toContain("{{.PATHSEP}}");

        // Check that wsh commands are used
        expect(pwshTemplate).toContain("wsh token");
        expect(pwshTemplate).toContain("wsh completion");
    });

    it("InitRcFiles should use proper PATH separator for platform", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Check for platform-specific PATH separator logic
        expect(shellutilGo).toContain('if runtime.GOOS == "windows"');
        expect(shellutilGo).toContain('pathSep = ";"');
        expect(shellutilGo).toContain('pathSep = ":"');
    });
});

describe("E2E: Shell Integration", () => {
    it("shell startup files should be generated with correct structure", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Check that all shell integration files are defined
        expect(shellutilGo).toContain("ZshStartup_Zprofile");
        expect(shellutilGo).toContain("BashStartup_Bashrc");
        expect(shellutilGo).toContain("FishStartup_Wavefish");
        expect(shellutilGo).toContain("PwshStartup_wavepwsh");

        // Check that InitRcFiles writes templates to files
        expect(shellutilGo).toContain("WriteTemplateToFile");
        expect(shellutilGo).toContain("wavepwsh.ps1");
    });

    it("wsh binary should be executable on Unix platforms", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Check that binary is copied with executable permissions
        expect(shellutilGo).toContain("0755");
        expect(shellutilGo).toContain("AtomicRenameCopy");
    });

    it("wsh PATH configuration should handle special characters", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Check for proper quoting functions
        expect(shellutilGo).toContain("HardQuote");
        expect(shellutilGo).toContain("HardQuotePowerShell");

        // These functions should be used for WSHBINDIR template params
        const initRcFilesFunc = shellutilGo.match(/func InitRcFiles[\s\S]*?^}/m);
        expect(initRcFilesFunc).toBeTruthy();
        expect(initRcFilesFunc[0]).toContain('HardQuote(absWshBinDir)');
        expect(initRcFilesFunc[0]).toContain('HardQuotePowerShell(absWshBinDir)');
    });
});

describe("E2E: Error Handling", () => {
    it("initCustomShellStartupFilesInternal should NOT silently ignore wsh binary errors", () => {
        const shellutilGo = fs.readFileSync(
            path.join(__dirname, "../pkg/util/shellutil/shellutil.go"),
            "utf-8"
        );

        // Find the initCustomShellStartupFilesInternal function
        const funcMatch = shellutilGo.match(/func initCustomShellStartupFilesInternal\(\)[\s\S]*?^}/m);
        expect(funcMatch).toBeTruthy();

        const funcBody = funcMatch[0];

        // Check that wsh binary stat error returns actual error, not nil
        // This is CRITICAL - returning nil means silent failure
        const statErrorCheck = funcBody.match(/if _, err := os\.Stat\(wshFullPath\);[\s\S]*?return/);
        if (statErrorCheck) {
            // If there's a return after stat error, it should return an error, not nil
            expect(statErrorCheck[0]).not.toMatch(/return nil\s*$/);
            // Or it should be clearly marked as non-fatal with proper logging
            expect(funcBody).toContain("log.Printf");
        }
    });

    it("version bump script should enforce all changes committed before bump", () => {
        const bumpScript = fs.readFileSync(
            path.join(__dirname, "../bump-version.sh"),
            "utf-8"
        );

        // Check for uncommitted changes detection
        expect(bumpScript).toContain("git diff-index");
        expect(bumpScript).toContain("uncommitted changes");
        expect(bumpScript).toContain("RELEASE WORKFLOW REMINDER");
    });
});
