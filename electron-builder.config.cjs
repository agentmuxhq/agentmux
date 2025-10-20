const { Arch } = require("electron-builder");
const pkg = require("./package.json");
const fs = require("fs");
const path = require("path");

const windowsShouldSign = !!process.env.SM_CODE_SIGNING_CERT_SHA1_HASH;

/**
 * Verify that required build artifacts exist before packaging
 * @throws {Error} if required files are missing
 */
function verifyRequiredArtifacts() {
    const version = pkg.version;

    // CRITICAL: All these files MUST exist before packaging
    // Missing any of these will cause runtime failures
    const requiredFiles = [
        // Frontend
        "dist/main/index.js",           // Main process entry point
        "dist/frontend/index.html",     // Frontend UI

        // Backend binaries (versioned for correct shell integration)
        "dist/bin/wavesrv.x64.exe",                    // Backend server
        `dist/bin/wsh-${version}-windows.x64.exe`,     // Shell integration (x64) - REQUIRED for PowerShell
        `dist/bin/wsh-${version}-windows.arm64.exe`,   // Shell integration (ARM64)
    ];

    const missingFiles = [];
    for (const file of requiredFiles) {
        if (!fs.existsSync(path.resolve(__dirname, file))) {
            missingFiles.push(file);
        }
    }

    if (missingFiles.length > 0) {
        const errorMsg = `
❌ BUILD FAILED: Required artifacts are missing!

Missing files:
${missingFiles.map((f) => `  - ${f}`).join("\n")}

CRITICAL: wsh binaries MUST match package.json version (${version})
Without versioned wsh, shell integration will fail with "wsh not found" errors.

Before packaging, you must:
1. Build the frontend: npm run build:prod
2. Build the Go binaries: task build:backend
3. Verify versions match: bash scripts/verify-version.sh

The package cannot be created without these critical files.
`;
        throw new Error(errorMsg);
    }

    console.log("✓ All required artifacts present for version", version);
}

// Run verification before configuration is used
verifyRequiredArtifacts();

/**
 * @type {import('electron-builder').Configuration}
 * @see https://www.electron.build/configuration/configuration
 */
const config = {
    appId: pkg.build.appId,
    productName: pkg.productName,
    executableName: pkg.productName,
    artifactName: "${productName}-${platform}-${arch}-${version}.${ext}",
    generateUpdatesFilesForAllChannels: true,
    npmRebuild: false,
    nodeGypRebuild: false,
    electronCompile: false,
    files: [
        "dist/**/*",
        "package.json",
        // Exclude unnecessary locale files to reduce build size (~35-40MB savings)
        "!**/locales/**/*",
        "**/locales/en-US.pak", // Only include English locale
    ],
    directories: {
        output: "make",
    },
    asarUnpack: [
        "dist/bin/**/*", // wavesrv and wsh binaries (platform-specific after filtering)
        "dist/docsite/**/*", // the static docsite
    ],
    mac: {
        target: [
            {
                target: "zip",
                arch: ["arm64", "x64"],
            },
            {
                target: "dmg",
                arch: ["arm64", "x64"],
            },
        ],
        // macOS-specific files: exclude cross-platform wsh binaries
        files: [
            "!dist/bin/wsh-*-windows.*",  // Exclude Windows binaries from macOS builds
            "!dist/bin/wsh-*-linux.*",    // Exclude Linux binaries from macOS builds
        ],
        category: "public.app-category.developer-tools",
        minimumSystemVersion: "10.15.0",
        mergeASARs: true,
        singleArchFiles: "**/dist/bin/wavesrv.*",
        entitlements: "build/entitlements.mac.plist",
        entitlementsInherit: "build/entitlements.mac.plist",
        extendInfo: {
            NSContactsUsageDescription: "A CLI application running in Wave wants to use your contacts.",
            NSRemindersUsageDescription: "A CLI application running in Wave wants to use your reminders.",
            NSLocationWhenInUseUsageDescription:
                "A CLI application running in Wave wants to use your location information while active.",
            NSLocationAlwaysUsageDescription:
                "A CLI application running in Wave wants to use your location information, even in the background.",
            NSCameraUsageDescription: "A CLI application running in Wave wants to use the camera.",
            NSMicrophoneUsageDescription: "A CLI application running in Wave wants to use your microphone.",
            NSCalendarsUsageDescription: "A CLI application running in Wave wants to use Calendar data.",
            NSLocationUsageDescription: "A CLI application running in Wave wants to use your location information.",
            NSAppleEventsUsageDescription: "A CLI application running in Wave wants to use AppleScript.",
        },
    },
    linux: {
        artifactName: "${name}-${platform}-${arch}-${version}.${ext}",
        // Linux-specific files: exclude cross-platform wsh binaries
        files: [
            "!dist/bin/wsh-*-windows.*",  // Exclude Windows binaries from Linux builds
            "!dist/bin/wsh-*-darwin.*",   // Exclude macOS binaries from Linux builds
        ],
        category: "TerminalEmulator",
        executableName: pkg.name,
        target: ["zip", "deb", "rpm", "snap", "AppImage", "pacman"],
        synopsis: pkg.description,
        description: null,
        desktop: {
            entry: {
                Name: pkg.productName,
                Comment: pkg.description,
                Keywords: "developer;terminal;emulator;",
                Categories: "Development;Utility;",
            },
        },
        executableArgs: ["--enable-features", "UseOzonePlatform", "--ozone-platform-hint", "auto"], // Hint Electron to use Ozone abstraction layer for native Wayland support
    },
    deb: {
        afterInstall: "build/deb-postinstall.tpl",
    },
    win: {
        target: ["nsis", "msi", "zip"],
        // Windows-specific files: exclude cross-platform wsh binaries (~84MB savings)
        files: [
            "!dist/bin/wsh-*-darwin.*",  // Exclude macOS binaries from Windows builds
            "!dist/bin/wsh-*-linux.*",   // Exclude Linux binaries from Windows builds
        ],
        signtoolOptions: windowsShouldSign && {
            signingHashAlgorithms: ["sha256"],
            publisherName: "Command Line Inc",
            certificateSubjectName: "Command Line Inc",
            certificateSha1: process.env.SM_CODE_SIGNING_CERT_SHA1_HASH,
        },
    },
    appImage: {
        license: "LICENSE",
    },
    snap: {
        base: "core22",
        confinement: "classic",
        allowNativeWayland: true,
        artifactName: "${name}_${version}_${arch}.${ext}",
    },
    rpm: {
        // this should remove /usr/lib/.build-id/ links which can conflict with other electron apps like slack
        fpm: ["--rpm-rpmbuild-define", "_build_id_links none"],
    },
    publish: {
        provider: "generic",
        url: "https://dl.waveterm.dev/releases-w2",
    },
    afterPack: (context) => {
        // This is a workaround to restore file permissions to the wavesrv binaries on macOS after packaging the universal binary.
        if (context.electronPlatformName === "darwin" && context.arch === Arch.universal) {
            const packageBinDir = path.resolve(
                context.appOutDir,
                `${pkg.productName}.app/Contents/Resources/app.asar.unpacked/dist/bin`
            );

            // Reapply file permissions to the wavesrv binaries in the final app package
            fs.readdirSync(packageBinDir, {
                recursive: true,
                withFileTypes: true,
            })
                .filter((f) => f.isFile() && f.name.startsWith("wavesrv"))
                .forEach((f) => fs.chmodSync(path.resolve(f.parentPath ?? f.path, f.name), 0o755)); // 0o755 corresponds to -rwxr-xr-x
        }
    },
};

module.exports = config;
