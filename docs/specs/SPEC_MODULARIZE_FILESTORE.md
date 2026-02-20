# Spec: Modularize filestore.rs

## Problem

`agentmuxsrv-rs/src/backend/storage/filestore.rs` is 1531 lines (986 code + 545 tests) in a single file. This makes it hard to navigate and will complicate the upcoming `spawn_blocking` refactor.

## Goal

Split `filestore.rs` into a `filestore/` directory module with focused submodules. External callers must not change — all public types are re-exported from `filestore/mod.rs`.

## Current Public API

Used externally (must stay accessible via `storage::filestore::*`):
- `FileStore` — struct (main.rs, server/mod.rs)
- `FileOpts` — struct (wavefileutil.rs)
- `FileMeta` — type alias (wavefileutil.rs)
- `WaveFile` — struct (wavefileutil.rs)
- `DEFAULT_FLUSH_SECS` — const

## New File Layout

```
backend/storage/filestore/
├── mod.rs          (~30 lines)  — re-exports, constants
├── types.rs        (~90 lines)  — wire types
├── cache.rs        (~20 lines)  — internal cache structs
├── core.rs         (~400 lines) — FileStore struct + CRUD ops
├── offset_ops.rs   (~170 lines) — write_at, read_at
├── ijson.rs        (~120 lines) — append_ijson, compact_ijson, thresholds
└── tests.rs        (~545 lines) — all #[cfg(test)] tests
```

## File Contents

### `mod.rs`
```rust
mod cache;
mod core;
mod ijson;
mod offset_ops;
mod types;
#[cfg(test)]
mod tests;

pub use types::{FileMeta, FileOpts, WaveFile};
pub use core::{FileStore, DEFAULT_FLUSH_SECS};
```

### `types.rs` (lines 29-88 of current file)
- `FileOpts` struct + serde
- `FileMeta` type alias
- `WaveFile` struct + impl (data_length, data_start_idx)
- Helper fns: `is_zero_i64`, `is_zero_i32`
- Imports: `serde`, `std::collections::HashMap`

### `cache.rs` (lines 90-103 of current file)
- `DataCacheEntry` struct (pub(super))
- `CacheEntry` struct (pub(super))
- Both are crate-internal, not publicly exported

### `core.rs` (lines 105-981 of current file, minus offset_ops and ijson sections)
Contains `FileStore` struct and these methods:
- `open()`, `open_in_memory()`, `configure_and_migrate()`
- `now_ms()` (private helper)
- `make_file()`, `delete_file()`, `delete_zone()`
- `stat()`
- `write_file()`, `read_file()`
- `append_data()`
- `list_files()`, `get_all_zone_ids()`
- `flush_cache()`
- `split_into_parts()` (private helper)
- `start_flusher()`

Imports from siblings:
```rust
use super::types::{FileMeta, FileOpts, WaveFile};
use super::cache::{CacheEntry, DataCacheEntry};
use crate::backend::storage::error::StoreError;
use crate::backend::storage::migrations::run_filestore_migrations;
```

### `offset_ops.rs` (lines 570-831 of current file)
`impl FileStore` block with:
- `write_at()` — write data at specific offset with circular file support
- `read_at()` — read data at specific offset with circular file adjustment

Imports from siblings:
```rust
use super::core::{FileStore, PART_DATA_SIZE};
use super::cache::DataCacheEntry;
use crate::backend::storage::error::StoreError;
```

Note: `PART_DATA_SIZE` must be `pub(super)` in core.rs for offset_ops.rs to access it.

### `ijson.rs` (lines 833-942 of current file)
`impl FileStore` block with:
- IJson constants (IJSON_NUM_COMMANDS, IJSON_INC_BYTES, thresholds)
- `append_ijson()` — append command + check compaction
- `compact_ijson()` — apply commands and rewrite file

Imports from siblings:
```rust
use super::core::FileStore;
use super::types::FileMeta;
use crate::backend::storage::error::StoreError;
```

### `tests.rs` (lines 987-1531 of current file)
Move entire `#[cfg(test)] mod tests { ... }` block.
Change `use super::*` to import from parent module:
```rust
use super::{FileStore, FileMeta, FileOpts, WaveFile};
use super::core::PART_DATA_SIZE;
use crate::backend::storage::error::StoreError;
```

## Key Implementation Notes

1. **`PART_DATA_SIZE`** is currently a private const in the file. It needs to be `pub(super)` in `core.rs` so `offset_ops.rs` and `tests.rs` can use it.

2. **`impl FileStore` across files**: Rust allows `impl` blocks in separate files as long as the struct is defined in the same crate. `core.rs` defines `FileStore`, and `offset_ops.rs` / `ijson.rs` add methods via additional `impl FileStore` blocks.

3. **Cache types are `pub(super)`**: `DataCacheEntry` and `CacheEntry` are only used within the filestore module — they don't need to be publicly exported.

4. **No external import changes**: `storage/mod.rs` already has `pub use filestore::FileStore`. When `filestore` becomes a directory module, `pub use filestore::FileStore` still resolves through `filestore/mod.rs`.

## Verification

1. `cargo build --release -p agentmuxsrv-rs` — compiles without errors
2. `cargo test -p agentmuxsrv-rs` — all existing tests pass
3. `grep -r 'filestore::' src/` — all external imports still resolve
