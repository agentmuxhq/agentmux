#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: ./scripts/update-tauri.sh <version>"
    echo "Example: ./scripts/update-tauri.sh 2.11.0"
    exit 1
fi

VERSION=$1
MAJOR_MINOR=$(echo $VERSION | cut -d. -f1,2)

echo "🔧 Updating all Tauri dependencies to $VERSION..."
echo

# Update package.json (remove ^ and pin exact versions)
echo "📝 Updating package.json..."
npm install \
    @tauri-apps/cli@$VERSION \
    @tauri-apps/api@$VERSION \
    --save-exact

# Update Cargo.toml
echo "📝 Updating Cargo.toml..."
sed -i.bak "s/tauri = { version = \"=[0-9.]*\"/tauri = { version = \"=$MAJOR_MINOR\"/" src-tauri/Cargo.toml
rm -f src-tauri/Cargo.toml.bak

# Update Cargo.lock
echo "📝 Updating Cargo.lock..."
cd src-tauri && cargo update tauri && cd ..

echo
echo "✅ Updated all Tauri dependencies to $VERSION"
echo

# Verify
echo "🔍 Verifying version alignment..."
./scripts/verify-tauri-versions.sh
