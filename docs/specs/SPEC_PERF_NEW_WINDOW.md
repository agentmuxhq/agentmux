# Spec: Optimize New Window Creation Performance

## Problem

Opening new instances is noticeably slower on the Rust backend (`agentmuxsrv-rs`) than the Go backend (`agentmuxsrv`). The goal is to shave every last bit of low performance possible.

### Root Cause Analysis

The `create_window` critical path performs **8 separate Mutex lock/unlock + SQLite I/O cycles**, each auto-committed independently. The frontend then makes **3 sequential RPC calls** before the window becomes interactive.

**Current critical path (Rust, ~25-50ms):**

```
Frontend                              Backend (agentmuxsrv-rs)
│
├─ RPC: GetClientData ──────────────→ lock → SELECT client → unlock      (~2ms)
│  ← response ──────────────────────
│
├─ RPC: CreateWindow("") ──────────→ create_workspace:
│                                      lock → INSERT workspace → unlock   (~3ms)
│                                    create_tab:
│                                      lock → SELECT workspace → unlock   (~1ms)
│                                      lock → INSERT layout → unlock      (~2ms)
│                                      lock → INSERT tab → unlock         (~2ms)
│                                      lock → UPDATE workspace → unlock   (~2ms)
│                                    create_window:
│                                      lock → INSERT window → unlock      (~3ms)
│  ← response ──────────────────────
│
├─ RPC: GetWorkspace ──────────────→ lock → SELECT workspace → unlock     (~2ms)
│  ← response ──────────────────────
│
├─ Load WOS data (layout, tabs) ──→ multiple lock/SELECT/unlock cycles
│
└─ Render UI
```

**Total: ~25-50ms backend, 3 network roundtrips, 8+ lock acquisitions**

### Why Go Is Faster

1. **Go uses `WithTx` (explicit transactions)** — wraps multiple operations in `BEGIN`/`COMMIT`, so SQLite only fsyncs once per transaction instead of per-statement
2. **Go's `CreateTab(isInitialLaunch=false)` applies `GetNewTabLayout()`** immediately, creating a terminal block — the tab is usable the moment it arrives. Rust creates a bare tab with no blocks, requiring extra async work.
3. **Go's `reflect`-based service dispatch** has overhead, but Go's `database/sql` connection pool handles `MaxOpenConns(1)` more efficiently than Rust's `Mutex<Connection>` pattern (Go queues at the pool level, Rust blocks the thread).

### Key Metrics

| Metric | Current (Rust) | Target | Go Baseline |
|--------|---------------|--------|-------------|
| Backend CreateWindow latency | ~20-35ms | <8ms | ~10-15ms |
| Total frontend-to-interactive | ~80-150ms | <40ms | ~40-60ms |
| Lock acquisitions per CreateWindow | 8+ | 1 | 1 (via WithTx) |
| Network roundtrips for new window | 3 | 1 | 3 (same) |

---

## Plan

### Phase 1: Add Transaction Support to WaveStore (HIGH IMPACT)

**Files:** `agentmuxsrv-rs/src/backend/storage/wstore.rs`

The single highest-impact change. Currently every `store.insert()` / `store.update()` acquires the Mutex independently and auto-commits. This means 8 separate fsyncs for one CreateWindow.

**Add a `with_tx` method:**

```rust
impl WaveStore {
    /// Execute multiple operations in a single SQLite transaction.
    /// Acquires the Mutex once, wraps ops in BEGIN/COMMIT.
    pub fn with_tx<F, R>(&self, f: F) -> Result<R, StoreError>
    where
        F: FnOnce(&Connection) -> Result<R, StoreError>,
    {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("BEGIN")?;
        match f(&conn) {
            Ok(result) => {
                conn.execute_batch("COMMIT")?;
                Ok(result)
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }
}
```

Add internal `_insert`, `_update`, `_get` variants that take `&Connection` directly (no Mutex acquisition) for use inside `with_tx` closures.

**Expected impact:** Reduces 8 lock acquisitions to 1, and 8 fsyncs to 1. ~60-70% reduction in CreateWindow latency.

### Phase 2: Batch CreateWindow Into Single Transaction

**Files:** `agentmuxsrv-rs/src/backend/wcore.rs`

Refactor `create_window()` to use the new `with_tx`:

```rust
pub fn create_window_batched(
    store: &WaveStore,
    workspace_id: &str,
) -> Result<(Window, Workspace, Tab), StoreError> {
    store.with_tx(|conn| {
        // All operations share one lock + one transaction
        let ws = insert_workspace_inner(conn, "", "", "")?;
        let (tab, layout) = insert_tab_inner(conn, &ws.oid, "", false)?;
        let window = insert_window_inner(conn, &ws.oid)?;
        // Update client in same transaction
        let mut client = get_client_inner(conn)?;
        client.windowids.push(window.oid.clone());
        update_client_inner(conn, &mut client)?;
        Ok((window, ws, tab))
    })
}
```

**Expected impact:** Single lock acquisition, single fsync, returns all data at once.

### Phase 3: Single "InitNewWindow" RPC Endpoint

**Files:**
- `agentmuxsrv-rs/src/server/service.rs` — add new method
- `frontend/wave.ts` — refactor `initTauriNewWindow()`
- `frontend/app/store/services.ts` — add new service method

Replace the 3 sequential RPCs with 1:

**Backend:** Add `("window", "InitNewWindow")` that returns `{ window, workspace, tab, client }` in a single response.

**Frontend:**
```typescript
// Before: 3 sequential RPCs
const clientData = await withTimeout(ClientService.GetClientData(), RPC_TIMEOUT);
const newWindow = await withTimeout(WindowService.CreateWindow(null, ""), RPC_TIMEOUT);
const workspace = await withTimeout(WorkspaceService.GetWorkspace(newWindow.workspaceid), RPC_TIMEOUT);

// After: 1 RPC
const { window, workspace, tab, client } = await withTimeout(
    WindowService.InitNewWindow(null),
    RPC_TIMEOUT
);
```

**Expected impact:** Eliminates 2 network roundtrips (~2-4ms saved), removes 2 additional lock acquisitions for GetClientData and GetWorkspace.

### Phase 4: Default Block Creation in create_tab

**Files:** `agentmuxsrv-rs/src/backend/wcore.rs`

Currently Rust's `create_tab` creates a bare tab with empty `blockids`. The Go backend applies `GetNewTabLayout()` which creates a terminal block. This is why widgets appear with a delay in new windows — they eventually appear via async frontend logic, but the initial render is blank.

**Add layout application in Rust (inside the transaction):**

```rust
fn create_tab_with_default_layout(
    conn: &Connection,
    ws_id: &str,
    is_initial_launch: bool,
) -> Result<Tab, StoreError> {
    let tab = insert_tab_inner(conn, ws_id, "", !is_initial_launch)?;

    if !is_initial_launch {
        // Apply new tab layout: 1 terminal block
        let meta = new_tab_layout_meta();  // {"view": "term", "controller": "shell"}
        insert_block_inner(conn, &tab.oid, meta)?;
    }
    // If initial launch, defer to BootstrapStarterLayout (TOS flow)

    Ok(tab)
}
```

**Expected impact:** Eliminates widget appearance delay in new windows. Tab is immediately usable with a terminal.

### Phase 5: SQLite PRAGMA Tuning

**Files:** `agentmuxsrv-rs/src/backend/storage/wstore.rs`

Add performance-oriented PRAGMAs:

```rust
conn.execute_batch(
    "PRAGMA journal_mode=WAL;
     PRAGMA busy_timeout=5000;
     PRAGMA synchronous=NORMAL;     -- NEW: WAL mode is crash-safe with NORMAL
     PRAGMA cache_size=-8000;       -- NEW: 8MB page cache (default is 2MB)
     PRAGMA mmap_size=268435456;    -- NEW: 256MB memory-mapped I/O
     PRAGMA temp_store=MEMORY;      -- NEW: temp tables in memory
     PRAGMA wal_autocheckpoint=1000; -- NEW: less frequent checkpointing
    "
)?;
```

Key change: `synchronous=NORMAL` with WAL mode is crash-safe and reduces fsync calls significantly.

**Expected impact:** ~30-50% reduction in per-operation I/O latency. Safe with WAL mode.

### Phase 6: Benchmarking Infrastructure

**Files:**
- `agentmuxsrv-rs/benches/wcore_bench.rs` (new)
- `agentmuxsrv-rs/Cargo.toml` (add criterion dev-dependency)

Add `criterion`-based benchmarks for the critical path:

```rust
fn bench_create_window(c: &mut Criterion) {
    let store = WaveStore::open_in_memory().unwrap();
    bootstrap_test_client(&store);

    c.bench_function("create_window_empty_workspace", |b| {
        b.iter(|| create_window(&store, ""))
    });

    c.bench_function("create_window_batched", |b| {
        b.iter(|| create_window_batched(&store, ""))
    });
}
```

Also add timing instrumentation to the HTTP service layer:

```rust
async fn handle_service(...) -> ... {
    let start = std::time::Instant::now();
    let result = dispatch_service(...);
    let elapsed = start.elapsed();
    tracing::info!(
        service = call.service,
        method = call.method,
        elapsed_ms = elapsed.as_millis(),
        "RPC completed"
    );
    result
}
```

**Expected impact:** Enables data-driven optimization decisions and regression detection.

### Phase 7: Modularize wcore.rs (Enables Future Optimization)

**Files:** `agentmuxsrv-rs/src/backend/wcore.rs` (748 lines → split)

Following the pattern from `SPEC_MODULARIZE_FILESTORE.md`:

```
backend/wcore/
├── mod.rs          (~30 lines)  — re-exports
├── client.rs       (~50 lines)  — get_client, ensure_client
├── workspace.rs    (~100 lines) — create/delete/list workspace
├── tab.rs          (~120 lines) — create/delete tab, layout application
├── window.rs       (~80 lines)  — create/close/focus window
├── block.rs        (~60 lines)  — create/delete block
├── batch.rs        (~100 lines) — batched operations (InitNewWindow, etc.)
└── constants.rs    (~60 lines)  — WORKSPACE_COLORS, ICONS, LAYOUT_ACTIONS
```

**Expected impact:** Cleaner separation enables targeted optimization of hot paths (window.rs, batch.rs) without touching unrelated code. Makes transaction boundaries explicit per-module.

---

## Implementation Order

| Phase | Effort | Impact | Dependencies |
|-------|--------|--------|-------------|
| 5. PRAGMA tuning | 15min | Medium | None |
| 1. WaveStore transactions | 2hr | **HIGH** | None |
| 2. Batch CreateWindow | 1hr | **HIGH** | Phase 1 |
| 6. Benchmarking | 1hr | Medium (enables measurement) | None |
| 4. Default blocks in tab | 1hr | Medium (UX) | Phase 2 |
| 3. Single InitNewWindow RPC | 2hr | Medium | Phase 2 |
| 7. Modularize wcore.rs | 1hr | Low (maintainability) | None |

**Recommended first PR:** Phases 5 + 1 + 2 + 6 together (one transaction-focused PR with benchmarks).

**Second PR:** Phases 4 + 3 (default layout + single RPC).

**Third PR:** Phase 7 (modularize, cleanup).

---

## Expected Results

| Metric | Before | After Phase 2 | After Phase 3 |
|--------|--------|---------------|---------------|
| Lock acquisitions | 8+ | 1 | 1 |
| SQLite fsyncs | 8 | 1 | 1 |
| Network roundtrips | 3 | 3 | 1 |
| Backend latency | ~25-50ms | ~5-10ms | ~5-10ms |
| Total window open time | ~80-150ms | ~30-50ms | ~20-35ms |
| Widget appearance | Delayed | Delayed | Immediate |

## Risks & Mitigations

- **Transaction rollback on partial failure:** If block creation fails, entire window creation rolls back cleanly (benefit of transactions).
- **`synchronous=NORMAL` durability:** WAL mode protects against corruption. Only risk is losing the last few milliseconds of writes on power loss — acceptable for a desktop app.
- **Single RPC breaking change:** Must maintain backward compatibility during transition. Keep old RPCs working, add `InitNewWindow` as new endpoint.
- **Default block creation parity:** Must match Go's `GetNewTabLayout()` exactly — currently `{view: "term", controller: "shell"}`. Verify with Go source.
