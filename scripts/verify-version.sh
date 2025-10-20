#!/usr/bin/env bash
# Verify all version references are consistent across the codebase

set -eo pipefail

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

success() { echo -e "${GREEN}✓ $1${NC}"; }
info() { echo -e "${CYAN}→ $1${NC}"; }
error() { echo -e "${RED}✗ $1${NC}" >&2; }
warn() { echo -e "${YELLOW}⚠ $1${NC}"; }

# Get current version from package.json
EXPECTED_VERSION=$(node -p "require('./package.json').version")
info "Checking version consistency for: $EXPECTED_VERSION"

ISSUES=0

# Check package-lock.json
LOCK_VERSION=$(node -p "require('./package-lock.json').version" 2>/dev/null || echo "")
if [[ "$LOCK_VERSION" != "$EXPECTED_VERSION" ]]; then
    error "package-lock.json version mismatch: $LOCK_VERSION (expected: $EXPECTED_VERSION)"
    ((ISSUES++))
else
    success "package-lock.json: $LOCK_VERSION"
fi

# Check version.cjs output
VERSION_CJS=$(node version.cjs)
if [[ "$VERSION_CJS" != "$EXPECTED_VERSION" ]]; then
    error "version.cjs output mismatch: $VERSION_CJS (expected: $EXPECTED_VERSION)"
    ((ISSUES++))
else
    success "version.cjs: $VERSION_CJS"
fi

# Check if wsh binaries match version
info "Checking wsh binaries..."
if ls dist/bin/wsh-${EXPECTED_VERSION}-*.exe 2>/dev/null | grep -q .; then
    success "wsh binaries found for version $EXPECTED_VERSION"
else
    warn "wsh binaries NOT found for version $EXPECTED_VERSION"
    warn "Run 'task build:backend' to rebuild binaries"
    echo "  Expected files like: dist/bin/wsh-${EXPECTED_VERSION}-windows.x64.exe"
fi

# Check VERSION_HISTORY.md
if grep -q "$EXPECTED_VERSION-fork" VERSION_HISTORY.md; then
    success "VERSION_HISTORY.md contains $EXPECTED_VERSION-fork"
else
    error "VERSION_HISTORY.md missing entry for $EXPECTED_VERSION-fork"
    ((ISSUES++))
fi

# Check for old hardcoded version references
info "Scanning for outdated version references..."
OUTDATED_REFS=$(grep -r "0\.1[0-9]\.[0-9]" \
    --include="*.ts" \
    --include="*.tsx" \
    --include="*.go" \
    --exclude-dir=node_modules \
    --exclude-dir=.git \
    --exclude-dir=dist \
    --exclude-dir=make \
    . 2>/dev/null | grep -v "$EXPECTED_VERSION" | grep -v "package-lock.json" || true)

if [[ -n "$OUTDATED_REFS" ]]; then
    warn "Found potential outdated version references in code:"
    echo "$OUTDATED_REFS" | head -10
    if [[ $(echo "$OUTDATED_REFS" | wc -l) -gt 10 ]]; then
        echo "  ... and $(( $(echo "$OUTDATED_REFS" | wc -l) - 10 )) more"
    fi
fi

echo ""
if [[ $ISSUES -eq 0 ]]; then
    success "All version checks passed! ✓"
    exit 0
else
    error "Found $ISSUES version inconsistencies"
    exit 1
fi
