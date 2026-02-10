#!/usr/bin/env node
// Generate latest.json updater manifest from release artifacts.
// Called in CI after downloading build artifacts and before creating the GitHub release.

const fs = require("fs");
const path = require("path");

const version = (process.env.GITHUB_REF_NAME || "").replace(/^v/, "");
if (!version) {
    console.error("GITHUB_REF_NAME not set or not a version tag");
    process.exit(1);
}

const artifactsDir = path.resolve("artifacts");
if (!fs.existsSync(artifactsDir)) {
    console.error(`Artifacts directory not found: ${artifactsDir}`);
    process.exit(1);
}

const baseUrl = `https://github.com/a5af/wavemux/releases/download/v${version}`;

// Recursively find all .sig files in artifacts
function findSigFiles(dir) {
    const results = [];
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            results.push(...findSigFiles(full));
        } else if (entry.name.endsWith(".sig")) {
            results.push(full);
        }
    }
    return results;
}

const sigFiles = findSigFiles(artifactsDir);
console.log(`Found ${sigFiles.length} signature files:`);
sigFiles.forEach((f) => console.log(`  ${path.relative(artifactsDir, f)}`));

// Helper: find a sig file matching a pattern and read its contents
function findSig(pattern) {
    const match = sigFiles.find((f) => path.basename(f).match(pattern));
    if (!match) return null;
    return {
        signature: fs.readFileSync(match, "utf-8").trim(),
        artifactName: path.basename(match).replace(/\.sig$/, ""),
    };
}

const platforms = {};

// Windows x86_64 — NSIS installer (.nsis.zip)
const winSig = findSig(/\.nsis\.zip\.sig$/);
if (winSig) {
    platforms["windows-x86_64"] = {
        signature: winSig.signature,
        url: `${baseUrl}/${winSig.artifactName}`,
    };
}

// macOS aarch64 — .app.tar.gz (arm64 build)
const macArmSig = findSig(/aarch64.*\.app\.tar\.gz\.sig$/) || findSig(/\.app\.tar\.gz\.sig$/);
if (macArmSig) {
    platforms["darwin-aarch64"] = {
        signature: macArmSig.signature,
        url: `${baseUrl}/${macArmSig.artifactName}`,
    };
}

// macOS x86_64 — .app.tar.gz (x64 build, may be separate or universal)
const macX64Sig = findSig(/x86_64.*\.app\.tar\.gz\.sig$/);
if (macX64Sig) {
    platforms["darwin-x86_64"] = {
        signature: macX64Sig.signature,
        url: `${baseUrl}/${macX64Sig.artifactName}`,
    };
} else if (macArmSig) {
    // If only one macOS build, use it for both architectures
    platforms["darwin-x86_64"] = {
        signature: macArmSig.signature,
        url: `${baseUrl}/${macArmSig.artifactName}`,
    };
}

// Linux x86_64 — AppImage.tar.gz
const linuxSig = findSig(/\.AppImage\.tar\.gz\.sig$/);
if (linuxSig) {
    platforms["linux-x86_64"] = {
        signature: linuxSig.signature,
        url: `${baseUrl}/${linuxSig.artifactName}`,
    };
}

if (Object.keys(platforms).length === 0) {
    console.error("No platform signatures found. Ensure TAURI_SIGNING_PRIVATE_KEY is set.");
    process.exit(1);
}

const manifest = {
    version,
    notes: `See release notes at https://github.com/a5af/wavemux/releases/tag/v${version}`,
    pub_date: new Date().toISOString(),
    platforms,
};

const outPath = path.resolve("latest.json");
fs.writeFileSync(outPath, JSON.stringify(manifest, null, 2) + "\n");
console.log(`\nGenerated ${outPath}:`);
console.log(JSON.stringify(manifest, null, 2));
