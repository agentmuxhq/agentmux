#!/bin/bash
set -e

# Change to project root
cd "$(dirname "$0")/.." || exit 1

echo "🔍 Checking Tauri version alignment..."
echo

# Extract versions
NPM_CLI=$(npm list @tauri-apps/cli --depth=0 --json 2>/dev/null | jq -r '.dependencies["@tauri-apps/cli"].version' || echo "not installed")
NPM_API=$(npm list @tauri-apps/api --depth=0 --json 2>/dev/null | jq -r '.dependencies["@tauri-apps/api"].version' || echo "not installed")
CARGO_TAURI=$(awk '/^\[\[package\]\]/{p=0} /^name = "tauri"$/{p=1} p && /^version =/{print $3; exit}' src-tauri/Cargo.lock | tr -d '"' || echo "not found")

# Extract major.minor
if [ "$NPM_CLI" != "not installed" ]; then
    NPM_CLI_MM=$(echo $NPM_CLI | cut -d. -f1,2)
else
    NPM_CLI_MM="N/A"
fi

if [ "$NPM_API" != "not installed" ]; then
    NPM_API_MM=$(echo $NPM_API | cut -d. -f1,2)
else
    NPM_API_MM="N/A"
fi

if [ "$CARGO_TAURI" != "not found" ]; then
    CARGO_MM=$(echo $CARGO_TAURI | cut -d. -f1,2)
else
    CARGO_MM="N/A"
fi

echo "📦 Installed Versions:"
echo "  @tauri-apps/cli: $NPM_CLI (major.minor: $NPM_CLI_MM)"
echo "  @tauri-apps/api: $NPM_API (major.minor: $NPM_API_MM)"
echo "  tauri crate:     $CARGO_TAURI (major.minor: $CARGO_MM)"
echo

# Verify all match
if [ "$NPM_CLI_MM" = "N/A" ] || [ "$NPM_API_MM" = "N/A" ] || [ "$CARGO_MM" = "N/A" ]; then
    echo "❌ ERROR: Some Tauri packages are not installed!"
    echo "   Run 'npm install' and 'cargo build' first."
    exit 1
fi

if [ "$NPM_CLI_MM" != "$NPM_API_MM" ] || [ "$NPM_CLI_MM" != "$CARGO_MM" ]; then
    echo "❌ ERROR: Tauri version mismatch!"
    echo
    echo "   All Tauri packages must be on the same major.minor version."
    echo "   Expected: All on $NPM_CLI_MM.x"
    echo
    echo "   To fix:"
    echo "   1. Update package.json to pin versions (remove ^)"
    echo "   2. Update Cargo.toml to pin version (use =MAJOR.MINOR)"
    echo "   3. Run: npm install && cd src-tauri && cargo update tauri"
    echo
    echo "   Or use: ./scripts/update-tauri.sh <version>"
    exit 1
fi

echo "✅ All Tauri versions aligned on $NPM_CLI_MM.x"
echo "   This build should succeed!"
