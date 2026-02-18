#!/usr/bin/env bash
# scripts/parity-test.sh — Go/Rust backend parity test
#
# Starts both backends, sends identical RPC calls, diffs JSON responses.
# Finds structural mismatches that cause frontend blank screen.
#
# Usage: bash scripts/parity-test.sh
# Requires: curl, jq

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ---- Config ----
GO_BIN="$PROJECT_ROOT/dist/bin/agentmuxsrv.arm64"
RS_BIN="$PROJECT_ROOT/dist/bin/agentmuxsrv-rs.arm64"
AUTH_KEY="parity-test-$(date +%s)"
TIMEOUT=15  # seconds to wait for backend startup

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ---- Helpers ----
log()  { echo -e "${CYAN}[parity]${NC} $*"; }
pass() { echo -e "${GREEN}  PASS${NC} $*"; }
fail() { echo -e "${RED}  FAIL${NC} $*"; }
warn() { echo -e "${YELLOW}  WARN${NC} $*"; }

cleanup() {
    log "Cleaning up..."
    exec 3>&- 2>/dev/null || true  # close Go stdin FIFO
    exec 4>&- 2>/dev/null || true  # close Rust stdin FIFO
    [ -n "${GO_PID:-}" ] && kill "$GO_PID" 2>/dev/null && wait "$GO_PID" 2>/dev/null || true
    [ -n "${RS_PID:-}" ] && kill "$RS_PID" 2>/dev/null && wait "$RS_PID" 2>/dev/null || true
    [ -d "${TMPDIR_BASE:-}" ] && rm -rf "$TMPDIR_BASE"
}
trap cleanup EXIT

# ---- Preflight ----
for cmd in curl jq; do
    command -v "$cmd" >/dev/null 2>&1 || { echo "ERROR: $cmd not found"; exit 1; }
done

if [ ! -x "$GO_BIN" ]; then
    echo "ERROR: Go binary not found at $GO_BIN"
    echo "Run: task build:backend"
    exit 1
fi

if [ ! -x "$RS_BIN" ]; then
    echo "ERROR: Rust binary not found at $RS_BIN"
    echo "Run: cd agentmuxsrv-rs && cargo build --release"
    exit 1
fi

# ---- Create isolated temp dirs ----
# Use short path to avoid Unix socket path length limit (104 chars on macOS)
TMPDIR_BASE=$(mktemp -d /tmp/parity.XXXX)
GO_DATA="$TMPDIR_BASE/go-data"
GO_CONFIG="$TMPDIR_BASE/go-config"
RS_DATA="$TMPDIR_BASE/rs-data"
RS_CONFIG="$TMPDIR_BASE/rs-config"

mkdir -p "$GO_DATA/instances/default/db" "$GO_CONFIG/instances/default"
mkdir -p "$RS_DATA/instances/default/db" "$RS_CONFIG/instances/default"

log "Temp dir: $TMPDIR_BASE"
log "Auth key: $AUTH_KEY"

# ---- Start Go backend ----
log "Starting Go backend..."
# Go backend exits when stdin closes, so keep stdin open with a FIFO
GO_FIFO="$TMPDIR_BASE/go-stdin"
mkfifo "$GO_FIFO"
WAVETERM_AUTH_KEY="$AUTH_KEY" \
WAVETERM_DATA_HOME="$GO_DATA" \
WAVETERM_CONFIG_HOME="$GO_CONFIG" \
WAVETERM_DEV=1 \
WCLOUD_ENDPOINT="https://api.waveterm.dev/central" \
WCLOUD_WS_ENDPOINT="wss://wsapi.waveterm.dev/" \
"$GO_BIN" < "$GO_FIFO" > "$TMPDIR_BASE/go-stdout.log" 2>"$TMPDIR_BASE/go-stderr.log" &
GO_PID=$!
# Keep the FIFO write end open so stdin doesn't close
exec 3>"$GO_FIFO"

# Wait for WAVESRV-ESTART
GO_WEB=""
for i in $(seq 1 $TIMEOUT); do
    if ! kill -0 "$GO_PID" 2>/dev/null; then
        echo "ERROR: Go backend died on startup"
        cat "$TMPDIR_BASE/go-stderr.log"
        exit 1
    fi
    if grep -q "WAVESRV-ESTART" "$TMPDIR_BASE/go-stderr.log" 2>/dev/null; then
        GO_WEB=$(grep "WAVESRV-ESTART" "$TMPDIR_BASE/go-stderr.log" | head -1 | grep -o 'web:[^ ]*' | cut -d: -f2-)
        break
    fi
    sleep 1
done

if [ -z "$GO_WEB" ]; then
    echo "ERROR: Go backend did not start within ${TIMEOUT}s"
    cat "$TMPDIR_BASE/go-stderr.log"
    exit 1
fi
log "Go backend ready at http://$GO_WEB"

# ---- Start Rust backend ----
log "Starting Rust backend..."
# Rust backend also exits when stdin closes
RS_FIFO="$TMPDIR_BASE/rs-stdin"
mkfifo "$RS_FIFO"
WAVETERM_AUTH_KEY="$AUTH_KEY" \
WAVETERM_DATA_HOME="$RS_DATA" \
WAVETERM_CONFIG_HOME="$RS_CONFIG" \
WAVETERM_DEV=1 \
WCLOUD_ENDPOINT="https://api.waveterm.dev/central" \
WCLOUD_WS_ENDPOINT="wss://wsapi.waveterm.dev/" \
"$RS_BIN" < "$RS_FIFO" > "$TMPDIR_BASE/rs-stdout.log" 2>"$TMPDIR_BASE/rs-stderr.log" &
RS_PID=$!
exec 4>"$RS_FIFO"

# Wait for WAVESRV-ESTART
RS_WEB=""
for i in $(seq 1 $TIMEOUT); do
    if ! kill -0 "$RS_PID" 2>/dev/null; then
        echo "ERROR: Rust backend died on startup"
        cat "$TMPDIR_BASE/rs-stderr.log"
        exit 1
    fi
    if grep -q "WAVESRV-ESTART" "$TMPDIR_BASE/rs-stderr.log" 2>/dev/null; then
        RS_WEB=$(grep "WAVESRV-ESTART" "$TMPDIR_BASE/rs-stderr.log" | head -1 | grep -o 'web:[^ ]*' | cut -d: -f2-)
        break
    fi
    sleep 1
done

if [ -z "$RS_WEB" ]; then
    echo "ERROR: Rust backend did not start within ${TIMEOUT}s"
    cat "$TMPDIR_BASE/rs-stderr.log"
    exit 1
fi
log "Rust backend ready at http://$RS_WEB"

# ---- RPC helpers ----
GO_URL="http://$GO_WEB/wave/service"
RS_URL="http://$RS_WEB/wave/service"

rpc_call() {
    local url="$1"
    local body="$2"
    curl -s -X POST "$url" \
        -H "Content-Type: application/json" \
        -H "X-AuthKey: $AUTH_KEY" \
        -d "$body"
}

# ---- JSON comparison functions ----

# Extract the structural "shape" of a JSON value: keys, types, nesting.
# Replaces UUIDs, timestamps, and version strings with placeholders.
normalize_json() {
    local json="$1"
    echo "$json" | jq -S '
        # Recursively walk and normalize values
        def normalize:
            if type == "string" then
                # UUID pattern
                if test("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$") then "<UUID>"
                # ORef pattern (type:uuid)
                elif test("^[a-z]+:[0-9a-f]{8}-") then "<OREF>"
                # Timestamp-like numbers as strings
                elif test("^[0-9]{10,}$") then "<TIMESTAMP>"
                # Version strings
                elif test("^[0-9]+\\.[0-9]+\\.[0-9]+") then "<VERSION>"
                else .
                end
            elif type == "number" then
                # Large numbers are likely timestamps
                if . > 1000000000000 then "<TIMESTAMP>"
                elif . > 1000000000 then "<TIMESTAMP>"
                else .
                end
            elif type == "array" then [.[] | normalize]
            elif type == "object" then
                # Remove version field (counters differ between independent databases)
                del(.version) |
                to_entries | map(.value |= normalize) | from_entries
            else .
            end;
        normalize
    '
}

# Compare two JSON responses and report differences
compare_responses() {
    local test_name="$1"
    local go_resp="$2"
    local rs_resp="$3"
    local go_file="$TMPDIR_BASE/${test_name}_go.json"
    local rs_file="$TMPDIR_BASE/${test_name}_rs.json"

    echo "$go_resp" > "$go_file"
    echo "$rs_resp" > "$rs_file"

    # Check if both are valid JSON
    if ! echo "$go_resp" | jq . >/dev/null 2>&1; then
        fail "$test_name: Go response is not valid JSON"
        echo "  Go response: $(echo "$go_resp" | head -c 200)"
        return 1
    fi
    if ! echo "$rs_resp" | jq . >/dev/null 2>&1; then
        fail "$test_name: Rust response is not valid JSON"
        echo "  Rust response: $(echo "$rs_resp" | head -c 200)"
        return 1
    fi

    # Compare success field
    local go_success rs_success
    go_success=$(echo "$go_resp" | jq '.success // false')
    rs_success=$(echo "$rs_resp" | jq '.success // false')
    if [ "$go_success" != "$rs_success" ]; then
        fail "$test_name: success field differs (Go=$go_success, Rust=$rs_success)"
        return 1
    fi

    # Check top-level keys
    local go_keys rs_keys
    go_keys=$(echo "$go_resp" | jq -S 'keys')
    rs_keys=$(echo "$rs_resp" | jq -S 'keys')
    if [ "$go_keys" != "$rs_keys" ]; then
        fail "$test_name: top-level keys differ"
        echo -e "    Go keys:   $go_keys"
        echo -e "    Rust keys: $rs_keys"
        # Continue to check data shape even if top-level keys differ
    fi

    # If error response, compare error presence
    if [ "$go_success" = "false" ]; then
        local go_has_error rs_has_error
        go_has_error=$(echo "$go_resp" | jq 'has("error")')
        rs_has_error=$(echo "$rs_resp" | jq 'has("error")')
        if [ "$go_has_error" != "$rs_has_error" ]; then
            fail "$test_name: error field presence differs"
            return 1
        fi
        pass "$test_name (both error)"
        return 0
    fi

    # Compare data shape (normalized)
    local go_norm rs_norm
    go_norm=$(normalize_json "$(echo "$go_resp" | jq '.data')")
    rs_norm=$(normalize_json "$(echo "$rs_resp" | jq '.data')")

    if [ "$go_norm" = "$rs_norm" ]; then
        pass "$test_name"
        return 0
    fi

    fail "$test_name: data shape mismatch"

    # Show detailed diff
    local go_norm_file="$TMPDIR_BASE/${test_name}_go_norm.json"
    local rs_norm_file="$TMPDIR_BASE/${test_name}_rs_norm.json"
    echo "$go_norm" > "$go_norm_file"
    echo "$rs_norm" > "$rs_norm_file"

    echo -e "${YELLOW}    --- Go (normalized) ---${NC}"
    echo -e "${YELLOW}    +++ Rust (normalized) +++${NC}"
    diff --unified=3 "$go_norm_file" "$rs_norm_file" | head -40 | while IFS= read -r line; do
        echo "    $line"
    done

    # Show key-level differences
    echo ""
    echo -e "    ${BOLD}Key differences in .data:${NC}"

    # Keys in Go but not Rust
    local go_data_keys rs_data_keys
    go_data_keys=$(echo "$go_resp" | jq -r '.data | if type == "object" then keys[] else empty end' 2>/dev/null || true)
    rs_data_keys=$(echo "$rs_resp" | jq -r '.data | if type == "object" then keys[] else empty end' 2>/dev/null || true)

    while IFS= read -r key; do
        [ -z "$key" ] && continue
        if ! echo "$rs_data_keys" | grep -qxF "$key"; then
            echo -e "    ${RED}  - Go has '$key' but Rust does not${NC}"
        fi
    done <<< "$go_data_keys"

    while IFS= read -r key; do
        [ -z "$key" ] && continue
        if ! echo "$go_data_keys" | grep -qxF "$key"; then
            echo -e "    ${RED}  - Rust has '$key' but Go does not${NC}"
        fi
    done <<< "$rs_data_keys"

    # Type differences for shared keys
    while IFS= read -r key; do
        [ -z "$key" ] && continue
        if echo "$rs_data_keys" | grep -qxF "$key"; then
            local go_type rs_type
            go_type=$(echo "$go_resp" | jq -r ".data.\"$key\" | type")
            rs_type=$(echo "$rs_resp" | jq -r ".data.\"$key\" | type")
            if [ "$go_type" != "$rs_type" ]; then
                echo -e "    ${RED}  - '$key': Go type=$go_type, Rust type=$rs_type${NC}"
            fi
        fi
    done <<< "$go_data_keys"

    return 1
}

# ====================================================================
# RPC Tests — matching the frontend init sequence from wave.ts
# ====================================================================

TOTAL=0
PASSED=0
FAILED=0

run_test() {
    local name="$1"
    local body="$2"
    TOTAL=$((TOTAL + 1))

    echo ""
    echo -e "${BOLD}Test $TOTAL: $name${NC}"

    local go_resp rs_resp
    go_resp=$(rpc_call "$GO_URL" "$body")
    rs_resp=$(rpc_call "$RS_URL" "$body")

    if compare_responses "$name" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
}

# Also dump the raw responses for inspection
run_test_raw() {
    local name="$1"
    local body="$2"
    TOTAL=$((TOTAL + 1))

    echo ""
    echo -e "${BOLD}Test $TOTAL: $name${NC}"

    local go_resp rs_resp
    go_resp=$(rpc_call "$GO_URL" "$body")
    rs_resp=$(rpc_call "$RS_URL" "$body")

    echo -e "  ${CYAN}Go response:${NC}"
    echo "$go_resp" | jq -S . 2>/dev/null | head -30 | sed 's/^/    /'
    echo -e "  ${CYAN}Rust response:${NC}"
    echo "$rs_resp" | jq -S . 2>/dev/null | head -30 | sed 's/^/    /'

    if compare_responses "$name" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
}

echo ""
echo -e "${BOLD}=====================================${NC}"
echo -e "${BOLD}  Go/Rust Backend Parity Tests${NC}"
echo -e "${BOLD}=====================================${NC}"

# ---- Test 1: GetClientData ----
# This is the first call the frontend makes. Critical.
run_test_raw "client.GetClientData" \
    '{"service":"client","method":"GetClientData","args":[]}'

# Extract window ID from Go response for subsequent tests
GO_CLIENT=$(rpc_call "$GO_URL" '{"service":"client","method":"GetClientData","args":[]}')
RS_CLIENT=$(rpc_call "$RS_URL" '{"service":"client","method":"GetClientData","args":[]}')

GO_WINDOW_ID=$(echo "$GO_CLIENT" | jq -r '.data.windowids[0] // empty')
RS_WINDOW_ID=$(echo "$RS_CLIENT" | jq -r '.data.windowids[0] // empty')

log "Go window ID: ${GO_WINDOW_ID:-none}"
log "Rust window ID: ${RS_WINDOW_ID:-none}"

# ---- Test 2: GetWindow ----
if [ -n "$GO_WINDOW_ID" ] && [ -n "$RS_WINDOW_ID" ]; then
    # Can't use same ID (different DBs), so test each with its own ID
    echo ""
    echo -e "${BOLD}Test: window.GetWindow (each backend's own window)${NC}"
    TOTAL=$((TOTAL + 1))

    go_resp=$(rpc_call "$GO_URL" "{\"service\":\"window\",\"method\":\"GetWindow\",\"args\":[\"$GO_WINDOW_ID\"]}")
    rs_resp=$(rpc_call "$RS_URL" "{\"service\":\"window\",\"method\":\"GetWindow\",\"args\":[\"$RS_WINDOW_ID\"]}")

    echo -e "  ${CYAN}Go response:${NC}"
    echo "$go_resp" | jq -S . 2>/dev/null | head -20 | sed 's/^/    /'
    echo -e "  ${CYAN}Rust response:${NC}"
    echo "$rs_resp" | jq -S . 2>/dev/null | head -20 | sed 's/^/    /'

    if compare_responses "window.GetWindow" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi

    # Extract workspace IDs
    GO_WS_ID=$(echo "$go_resp" | jq -r '.data.workspaceid // empty')
    RS_WS_ID=$(echo "$rs_resp" | jq -r '.data.workspaceid // empty')
    log "Go workspace ID: ${GO_WS_ID:-none}"
    log "Rust workspace ID: ${RS_WS_ID:-none}"
else
    warn "Skipping GetWindow — no window IDs found"
    # Try CreateWindow instead
    run_test_raw "window.CreateWindow" \
        '{"service":"window","method":"CreateWindow","args":[null, "", ""]}'

    GO_WIN_RESP=$(rpc_call "$GO_URL" '{"service":"window","method":"CreateWindow","args":[null, "", ""]}')
    RS_WIN_RESP=$(rpc_call "$RS_URL" '{"service":"window","method":"CreateWindow","args":[null, "", ""]}')
    GO_WINDOW_ID=$(echo "$GO_WIN_RESP" | jq -r '.data.oid // empty')
    RS_WINDOW_ID=$(echo "$RS_WIN_RESP" | jq -r '.data.oid // empty')
    GO_WS_ID=$(echo "$GO_WIN_RESP" | jq -r '.data.workspaceid // empty')
    RS_WS_ID=$(echo "$RS_WIN_RESP" | jq -r '.data.workspaceid // empty')
fi

# ---- Test 3: GetWorkspace ----
if [ -n "${GO_WS_ID:-}" ] && [ -n "${RS_WS_ID:-}" ]; then
    echo ""
    echo -e "${BOLD}Test: workspace.GetWorkspace (each backend's own workspace)${NC}"
    TOTAL=$((TOTAL + 1))

    go_resp=$(rpc_call "$GO_URL" "{\"service\":\"workspace\",\"method\":\"GetWorkspace\",\"args\":[\"$GO_WS_ID\"]}")
    rs_resp=$(rpc_call "$RS_URL" "{\"service\":\"workspace\",\"method\":\"GetWorkspace\",\"args\":[\"$RS_WS_ID\"]}")

    echo -e "  ${CYAN}Go response:${NC}"
    echo "$go_resp" | jq -S . 2>/dev/null | head -25 | sed 's/^/    /'
    echo -e "  ${CYAN}Rust response:${NC}"
    echo "$rs_resp" | jq -S . 2>/dev/null | head -25 | sed 's/^/    /'

    if compare_responses "workspace.GetWorkspace" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi

    # Extract tab IDs
    GO_TAB_ID=$(echo "$go_resp" | jq -r '.data.activetabid // .data.tabids[0] // empty')
    RS_TAB_ID=$(echo "$rs_resp" | jq -r '.data.activetabid // .data.tabids[0] // empty')
    log "Go active tab ID: ${GO_TAB_ID:-none}"
    log "Rust active tab ID: ${RS_TAB_ID:-none}"
else
    warn "Skipping GetWorkspace — no workspace IDs found"
fi

# ---- Test 4: GetObject (tab) ----
if [ -n "${GO_TAB_ID:-}" ] && [ -n "${RS_TAB_ID:-}" ]; then
    echo ""
    echo -e "${BOLD}Test: object.GetObject tab (each backend's own tab)${NC}"
    TOTAL=$((TOTAL + 1))

    go_resp=$(rpc_call "$GO_URL" "{\"service\":\"object\",\"method\":\"GetObject\",\"args\":[\"tab:$GO_TAB_ID\"]}")
    rs_resp=$(rpc_call "$RS_URL" "{\"service\":\"object\",\"method\":\"GetObject\",\"args\":[\"tab:$RS_TAB_ID\"]}")

    echo -e "  ${CYAN}Go response:${NC}"
    echo "$go_resp" | jq -S . 2>/dev/null | head -25 | sed 's/^/    /'
    echo -e "  ${CYAN}Rust response:${NC}"
    echo "$rs_resp" | jq -S . 2>/dev/null | head -25 | sed 's/^/    /'

    if compare_responses "object.GetObject(tab)" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi

    # Extract layout state ID from tab
    GO_LAYOUT_ID=$(echo "$go_resp" | jq -r '.data.layoutstate // empty')
    RS_LAYOUT_ID=$(echo "$rs_resp" | jq -r '.data.layoutstate // empty')
fi

# ---- Test 5: GetObject (layout) ----
if [ -n "${GO_LAYOUT_ID:-}" ] && [ -n "${RS_LAYOUT_ID:-}" ]; then
    echo ""
    echo -e "${BOLD}Test: object.GetObject layout (each backend's own layout)${NC}"
    TOTAL=$((TOTAL + 1))

    go_resp=$(rpc_call "$GO_URL" "{\"service\":\"object\",\"method\":\"GetObject\",\"args\":[\"layout:$GO_LAYOUT_ID\"]}")
    rs_resp=$(rpc_call "$RS_URL" "{\"service\":\"object\",\"method\":\"GetObject\",\"args\":[\"layout:$RS_LAYOUT_ID\"]}")

    echo -e "  ${CYAN}Go response:${NC}"
    echo "$go_resp" | jq -S . 2>/dev/null | head -30 | sed 's/^/    /'
    echo -e "  ${CYAN}Rust response:${NC}"
    echo "$rs_resp" | jq -S . 2>/dev/null | head -30 | sed 's/^/    /'

    if compare_responses "object.GetObject(layout)" "$go_resp" "$rs_resp"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
fi

# ---- Test 6: ListWorkspaces ----
run_test_raw "workspace.ListWorkspaces" \
    '{"service":"workspace","method":"ListWorkspaces","args":[]}'

# ---- Test 7: GetAllConnStatus ----
run_test "client.GetAllConnStatus" \
    '{"service":"client","method":"GetAllConnStatus","args":[]}'

# ---- Test 8: filestore.ReadFile (config) ----
run_test "filestore.ReadFile(settings)" \
    '{"service":"filestore","method":"ReadFile","args":["config/settings.json"]}'

# ====================================================================
# Summary
# ====================================================================

echo ""
echo -e "${BOLD}=====================================${NC}"
echo -e "${BOLD}  Results: $PASSED/$TOTAL passed${NC}"
if [ "$FAILED" -gt 0 ]; then
    echo -e "${RED}  $FAILED test(s) FAILED${NC}"
fi
echo -e "${BOLD}=====================================${NC}"
echo ""
echo "Raw responses saved to: $TMPDIR_BASE"
echo "  (cleanup on exit, copy files if you need them)"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
exit 0
