// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! FileStore: file storage with write-through cache + background flusher.
//! Port of Go's pkg/filestore/blockstore.go, blockstore_cache.go, blockstore_dbops.go.
//!
//! - Separate SQLite DB from WaveStore (matching Go).
//! - 64KB parts for efficient partial reads/writes.
//! - Write-through cache with periodic flush (5s default).
//! - Background flusher via `tokio::spawn` + `tokio::time::interval`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::error::StoreError;
use super::migrations::run_filestore_migrations;

/// Default part size: 64KB (matches Go's DefaultPartDataSize).
const PART_DATA_SIZE: usize = 64 * 1024;

/// Default flush interval in seconds.
pub const DEFAULT_FLUSH_SECS: u64 = 5;

// ---- Wire types (matching Go's wshrpc types) ----

/// File options. Matches Go's `FileOpts`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileOpts {
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub maxsize: i64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub circular: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ijson: bool,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub ijsonbudget: i32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncate: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub append: bool,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}
fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

/// File metadata. Matches Go's `FileMeta = map[string]any`.
pub type FileMeta = HashMap<String, serde_json::Value>;

/// A wave file record. Matches Go's `WaveFile`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveFile {
    pub zoneid: String,
    pub name: String,
    pub size: i64,
    pub createdts: i64,
    pub modts: i64,
    pub opts: FileOpts,
    pub meta: FileMeta,
}

impl WaveFile {
    /// Effective data length, accounting for circular files.
    pub fn data_length(&self) -> i64 {
        if self.opts.circular && self.opts.maxsize > 0 && self.size > self.opts.maxsize {
            self.opts.maxsize
        } else {
            self.size
        }
    }

    /// Start index of data for circular files.
    pub fn data_start_idx(&self) -> i64 {
        if self.opts.circular && self.opts.maxsize > 0 && self.size > self.opts.maxsize {
            self.size - self.opts.maxsize
        } else {
            0
        }
    }
}

/// Cache entry for file data parts.
#[derive(Debug, Clone)]
struct DataCacheEntry {
    part_idx: i32,
    data: Vec<u8>,
}

/// Cache entry for a file + its data parts.
#[derive(Debug)]
struct CacheEntry {
    file: Option<WaveFile>,
    data_entries: HashMap<i32, DataCacheEntry>,
    dirty: bool,
}

// ---- FileStore ----

/// SQLite-backed file storage with write-through cache.
pub struct FileStore {
    conn: Mutex<Connection>,
    cache: Mutex<HashMap<(String, String), CacheEntry>>,
}

impl FileStore {
    /// Open a FileStore backed by a file on disk.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        Self::configure_and_migrate(conn)
    }

    /// Open an in-memory FileStore for testing.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        Self::configure_and_migrate(conn)
    }

    fn configure_and_migrate(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;",
        )?;
        run_filestore_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            cache: Mutex::new(HashMap::new()),
        })
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// Create a new file. Fails if file already exists.
    pub fn make_file(
        &self,
        zone_id: &str,
        name: &str,
        meta: FileMeta,
        opts: FileOpts,
    ) -> Result<(), StoreError> {
        let now = Self::now_ms();
        let file = WaveFile {
            zoneid: zone_id.to_string(),
            name: name.to_string(),
            size: 0,
            createdts: now,
            modts: now,
            opts,
            meta,
        };

        let conn = self.conn.lock().unwrap();
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM db_wave_file WHERE zoneid = ?1 AND name = ?2",
                params![zone_id, name],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exists {
            return Err(StoreError::AlreadyExists);
        }

        let opts_json = serde_json::to_string(&file.opts)?;
        let meta_json = serde_json::to_string(&file.meta)?;
        conn.execute(
            "INSERT INTO db_wave_file (zoneid, name, size, createdts, modts, opts, meta) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![file.zoneid, file.name, file.size, file.createdts, file.modts, opts_json, meta_json],
        )?;

        // Add to cache
        let key = (zone_id.to_string(), name.to_string());
        let mut cache = self.cache.lock().unwrap();
        cache.insert(
            key,
            CacheEntry {
                file: Some(file),
                data_entries: HashMap::new(),
                dirty: false,
            },
        );

        Ok(())
    }

    /// Delete a file and all its data parts.
    pub fn delete_file(&self, zone_id: &str, name: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM db_wave_file WHERE zoneid = ?1 AND name = ?2",
            params![zone_id, name],
        )?;
        conn.execute(
            "DELETE FROM db_file_data WHERE zoneid = ?1 AND name = ?2",
            params![zone_id, name],
        )?;
        drop(conn);

        // Remove from cache
        let key = (zone_id.to_string(), name.to_string());
        let mut cache = self.cache.lock().unwrap();
        cache.remove(&key);

        Ok(())
    }

    /// Delete all files in a zone.
    pub fn delete_zone(&self, zone_id: &str) -> Result<(), StoreError> {
        // Get file names first for cache cleanup
        let names: Vec<String> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT name FROM db_wave_file WHERE zoneid = ?1")?;
            let rows = stmt.query_map(params![zone_id], |row| row.get(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM db_wave_file WHERE zoneid = ?1",
            params![zone_id],
        )?;
        conn.execute(
            "DELETE FROM db_file_data WHERE zoneid = ?1",
            params![zone_id],
        )?;
        drop(conn);

        let mut cache = self.cache.lock().unwrap();
        for name in names {
            cache.remove(&(zone_id.to_string(), name));
        }

        Ok(())
    }

    /// Get file metadata. Returns None if file doesn't exist.
    pub fn stat(&self, zone_id: &str, name: &str) -> Result<Option<WaveFile>, StoreError> {
        // Check cache first
        let key = (zone_id.to_string(), name.to_string());
        {
            let cache = self.cache.lock().unwrap();
            if let Some(entry) = cache.get(&key) {
                return Ok(entry.file.clone());
            }
        }

        // Load from DB
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT zoneid, name, size, createdts, modts, opts, meta FROM db_wave_file WHERE zoneid = ?1 AND name = ?2",
            params![zone_id, name],
            |row| {
                let opts_str: String = row.get(5)?;
                let meta_str: String = row.get(6)?;
                Ok(WaveFile {
                    zoneid: row.get(0)?,
                    name: row.get(1)?,
                    size: row.get(2)?,
                    createdts: row.get(3)?,
                    modts: row.get(4)?,
                    opts: serde_json::from_str(&opts_str).unwrap_or_default(),
                    meta: serde_json::from_str(&meta_str).unwrap_or_default(),
                })
            },
        );

        match result {
            Ok(file) => Ok(Some(file)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StoreError::Sqlite(e)),
        }
    }

    /// Write (replace) entire file contents.
    pub fn write_file(
        &self,
        zone_id: &str,
        name: &str,
        data: &[u8],
    ) -> Result<(), StoreError> {
        let key = (zone_id.to_string(), name.to_string());
        let now = Self::now_ms();

        // Split data into parts
        let parts = Self::split_into_parts(data);

        // Write directly to DB (write-through for full writes, matching Go's WriteFile)
        let conn = self.conn.lock().unwrap();

        // Verify file exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM db_wave_file WHERE zoneid = ?1 AND name = ?2",
                params![zone_id, name],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            return Err(StoreError::NotFound);
        }

        // Update file size
        conn.execute(
            "UPDATE db_wave_file SET size = ?1, modts = ?2 WHERE zoneid = ?3 AND name = ?4",
            params![data.len() as i64, now, zone_id, name],
        )?;

        // Replace all data parts
        conn.execute(
            "DELETE FROM db_file_data WHERE zoneid = ?1 AND name = ?2",
            params![zone_id, name],
        )?;
        for (idx, part_data) in parts.iter().enumerate() {
            conn.execute(
                "INSERT INTO db_file_data (zoneid, name, partidx, data) VALUES (?1, ?2, ?3, ?4)",
                params![zone_id, name, idx as i32, part_data],
            )?;
        }
        drop(conn);

        // Update cache
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(&key) {
            if let Some(ref mut file) = entry.file {
                file.size = data.len() as i64;
                file.modts = now;
            }
            entry.data_entries.clear();
            for (idx, part_data) in parts.into_iter().enumerate() {
                entry.data_entries.insert(
                    idx as i32,
                    DataCacheEntry {
                        part_idx: idx as i32,
                        data: part_data,
                    },
                );
            }
            entry.dirty = false;
        }

        Ok(())
    }

    /// Read entire file contents.
    pub fn read_file(&self, zone_id: &str, name: &str) -> Result<Option<Vec<u8>>, StoreError> {
        // Get file metadata
        let file = match self.stat(zone_id, name)? {
            Some(f) => f,
            None => return Ok(None),
        };

        if file.size == 0 {
            return Ok(Some(Vec::new()));
        }

        let data_len = file.data_length();
        let start_idx = file.data_start_idx();
        let num_parts = ((start_idx + data_len - 1) / PART_DATA_SIZE as i64 + 1) as i32;
        let start_part = (start_idx / PART_DATA_SIZE as i64) as i32;

        // Load parts from DB
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT partidx, data FROM db_file_data WHERE zoneid = ?1 AND name = ?2 ORDER BY partidx",
        )?;
        let rows = stmt.query_map(params![zone_id, name], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;

        let mut parts_map: HashMap<i32, Vec<u8>> = HashMap::new();
        for row in rows {
            let (idx, data) = row?;
            parts_map.insert(idx, data);
        }
        drop(stmt);
        drop(conn);

        // Assemble data
        let mut result = Vec::with_capacity(data_len as usize);
        for part_idx in start_part..start_part + num_parts {
            if let Some(part_data) = parts_map.get(&part_idx) {
                let part_start = part_idx as i64 * PART_DATA_SIZE as i64;
                let skip = if part_start < start_idx {
                    (start_idx - part_start) as usize
                } else {
                    0
                };
                let remaining = data_len as usize - result.len();
                let take = remaining.min(part_data.len() - skip);
                result.extend_from_slice(&part_data[skip..skip + take]);
            }
        }

        let _ = (num_parts, start_part); // used in loop above
        Ok(Some(result))
    }

    /// Append data to the end of a file.
    pub fn append_data(
        &self,
        zone_id: &str,
        name: &str,
        data: &[u8],
    ) -> Result<(), StoreError> {
        if data.is_empty() {
            return Ok(());
        }

        let key = (zone_id.to_string(), name.to_string());
        let now = Self::now_ms();

        let file = self.stat(zone_id, name)?.ok_or(StoreError::NotFound)?;
        let new_size = file.size + data.len() as i64;

        // Figure out which part to start writing at
        let start_offset = file.size;
        let start_part = (start_offset / PART_DATA_SIZE as i64) as i32;
        let offset_in_part = (start_offset % PART_DATA_SIZE as i64) as usize;

        // Load the last part if we need to append to it
        let conn = self.conn.lock().unwrap();
        let mut data_offset = 0usize;
        let mut current_part = start_part;

        if offset_in_part > 0 {
            // Load existing partial part
            let existing: Option<Vec<u8>> = conn
                .query_row(
                    "SELECT data FROM db_file_data WHERE zoneid = ?1 AND name = ?2 AND partidx = ?3",
                    params![zone_id, name, start_part],
                    |row| row.get(0),
                )
                .ok();

            let mut part_data = existing.unwrap_or_default();
            let space = PART_DATA_SIZE - part_data.len();
            let to_copy = space.min(data.len());
            part_data.extend_from_slice(&data[..to_copy]);
            data_offset = to_copy;

            conn.execute(
                "REPLACE INTO db_file_data (zoneid, name, partidx, data) VALUES (?1, ?2, ?3, ?4)",
                params![zone_id, name, current_part, part_data],
            )?;
            current_part += 1;
        }

        // Write remaining full parts
        while data_offset < data.len() {
            let end = (data_offset + PART_DATA_SIZE).min(data.len());
            let part_data = &data[data_offset..end];
            conn.execute(
                "REPLACE INTO db_file_data (zoneid, name, partidx, data) VALUES (?1, ?2, ?3, ?4)",
                params![zone_id, name, current_part, part_data],
            )?;
            data_offset = end;
            current_part += 1;
        }

        // Update file size
        conn.execute(
            "UPDATE db_wave_file SET size = ?1, modts = ?2 WHERE zoneid = ?3 AND name = ?4",
            params![new_size, now, zone_id, name],
        )?;
        drop(conn);

        // Update cache
        let mut cache = self.cache.lock().unwrap();
        if let Some(entry) = cache.get_mut(&key) {
            if let Some(ref mut f) = entry.file {
                f.size = new_size;
                f.modts = now;
            }
        }

        Ok(())
    }

    /// List all files in a zone.
    pub fn list_files(&self, zone_id: &str) -> Result<Vec<WaveFile>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT zoneid, name, size, createdts, modts, opts, meta FROM db_wave_file WHERE zoneid = ?1",
        )?;
        let rows = stmt.query_map(params![zone_id], |row| {
            let opts_str: String = row.get(5)?;
            let meta_str: String = row.get(6)?;
            Ok(WaveFile {
                zoneid: row.get(0)?,
                name: row.get(1)?,
                size: row.get(2)?,
                createdts: row.get(3)?,
                modts: row.get(4)?,
                opts: serde_json::from_str(&opts_str).unwrap_or_default(),
                meta: serde_json::from_str(&meta_str).unwrap_or_default(),
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::Sqlite)
    }

    /// Get all zone IDs that have files.
    pub fn get_all_zone_ids(&self) -> Result<Vec<String>, StoreError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT zoneid FROM db_wave_file")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::Sqlite)
    }

    /// Flush all dirty cache entries to the database.
    /// Returns (files_flushed, parts_flushed).
    pub fn flush_cache(&self) -> Result<(usize, usize), StoreError> {
        let dirty_keys: Vec<(String, String)> = {
            let cache = self.cache.lock().unwrap();
            cache
                .iter()
                .filter(|(_, e)| e.dirty)
                .map(|(k, _)| k.clone())
                .collect()
        };

        let mut files_flushed = 0;
        let mut parts_flushed = 0;

        for key in dirty_keys {
            let entry = {
                let mut cache = self.cache.lock().unwrap();
                cache.remove(&key)
            };

            if let Some(entry) = entry {
                if let Some(ref file) = entry.file {
                    let conn = self.conn.lock().unwrap();
                    let meta_json = serde_json::to_string(&file.meta)?;
                    conn.execute(
                        "UPDATE db_wave_file SET size = ?1, modts = ?2, meta = ?3 WHERE zoneid = ?4 AND name = ?5",
                        params![file.size, file.modts, meta_json, file.zoneid, file.name],
                    )?;

                    for data_entry in entry.data_entries.values() {
                        conn.execute(
                            "REPLACE INTO db_file_data (zoneid, name, partidx, data) VALUES (?1, ?2, ?3, ?4)",
                            params![file.zoneid, file.name, data_entry.part_idx, data_entry.data],
                        )?;
                        parts_flushed += 1;
                    }
                    files_flushed += 1;
                }
            }
        }

        Ok((files_flushed, parts_flushed))
    }

    // ---- Internal helpers ----

    /// Split data into PART_DATA_SIZE chunks.
    fn split_into_parts(data: &[u8]) -> Vec<Vec<u8>> {
        if data.is_empty() {
            return Vec::new();
        }
        data.chunks(PART_DATA_SIZE)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Start background flusher (call from async context).
    pub fn start_flusher(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let store = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(DEFAULT_FLUSH_SECS));
            loop {
                interval.tick().await;
                match store.flush_cache() {
                    Ok((files, parts)) => {
                        if files > 0 {
                            tracing::debug!(
                                "filestore flush: {} files, {} parts",
                                files,
                                parts
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("filestore flush error: {}", e);
                    }
                }
            }
        })
    }
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> FileStore {
        FileStore::open_in_memory().unwrap()
    }

    #[test]
    fn test_create_and_stat() {
        let store = make_store();
        store
            .make_file("zone1", "test.txt", FileMeta::new(), FileOpts::default())
            .unwrap();

        let file = store.stat("zone1", "test.txt").unwrap().unwrap();
        assert_eq!(file.zoneid, "zone1");
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.size, 0);
        assert!(file.createdts > 0);
    }

    #[test]
    fn test_create_duplicate_fails() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();
        let result = store.make_file("z1", "f1", FileMeta::new(), FileOpts::default());
        assert!(matches!(result, Err(StoreError::AlreadyExists)));
    }

    #[test]
    fn test_write_and_read() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();

        let data = b"hello world";
        store.write_file("z1", "f1", data).unwrap();

        let read_data = store.read_file("z1", "f1").unwrap().unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_write_large_file() {
        let store = make_store();
        store
            .make_file("z1", "big", FileMeta::new(), FileOpts::default())
            .unwrap();

        // Write data larger than one part (64KB)
        let data: Vec<u8> = (0..PART_DATA_SIZE * 3 + 1000)
            .map(|i| (i % 256) as u8)
            .collect();
        store.write_file("z1", "big", &data).unwrap();

        let read_data = store.read_file("z1", "big").unwrap().unwrap();
        assert_eq!(read_data.len(), data.len());
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_append_data() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();

        store.write_file("z1", "f1", b"hello").unwrap();
        store.append_data("z1", "f1", b" world").unwrap();

        let data = store.read_file("z1", "f1").unwrap().unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn test_append_across_parts() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();

        // Write nearly a full part
        let initial: Vec<u8> = vec![0xAA; PART_DATA_SIZE - 10];
        store.write_file("z1", "f1", &initial).unwrap();

        // Append across boundary
        let append_data: Vec<u8> = vec![0xBB; 100];
        store.append_data("z1", "f1", &append_data).unwrap();

        let data = store.read_file("z1", "f1").unwrap().unwrap();
        assert_eq!(data.len(), PART_DATA_SIZE - 10 + 100);
        assert_eq!(&data[..PART_DATA_SIZE - 10], &initial[..]);
        assert_eq!(&data[PART_DATA_SIZE - 10..], &append_data[..]);
    }

    #[test]
    fn test_delete_file() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();
        store.write_file("z1", "f1", b"data").unwrap();

        store.delete_file("z1", "f1").unwrap();
        assert!(store.stat("z1", "f1").unwrap().is_none());
        assert!(store.read_file("z1", "f1").unwrap().is_none());
    }

    #[test]
    fn test_delete_zone() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();
        store
            .make_file("z1", "f2", FileMeta::new(), FileOpts::default())
            .unwrap();
        store
            .make_file("z2", "f3", FileMeta::new(), FileOpts::default())
            .unwrap();

        store.delete_zone("z1").unwrap();

        assert!(store.stat("z1", "f1").unwrap().is_none());
        assert!(store.stat("z1", "f2").unwrap().is_none());
        assert!(store.stat("z2", "f3").unwrap().is_some()); // Different zone
    }

    #[test]
    fn test_list_files() {
        let store = make_store();
        store
            .make_file("z1", "a.txt", FileMeta::new(), FileOpts::default())
            .unwrap();
        store
            .make_file("z1", "b.txt", FileMeta::new(), FileOpts::default())
            .unwrap();
        store
            .make_file("z2", "c.txt", FileMeta::new(), FileOpts::default())
            .unwrap();

        let files = store.list_files("z1").unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_get_all_zone_ids() {
        let store = make_store();
        store
            .make_file("z1", "f1", FileMeta::new(), FileOpts::default())
            .unwrap();
        store
            .make_file("z2", "f2", FileMeta::new(), FileOpts::default())
            .unwrap();

        let mut zones = store.get_all_zone_ids().unwrap();
        zones.sort();
        assert_eq!(zones, vec!["z1", "z2"]);
    }

    #[test]
    fn test_stat_nonexistent() {
        let store = make_store();
        assert!(store.stat("z1", "nope").unwrap().is_none());
    }

    #[test]
    fn test_read_empty_file() {
        let store = make_store();
        store
            .make_file("z1", "empty", FileMeta::new(), FileOpts::default())
            .unwrap();
        let data = store.read_file("z1", "empty").unwrap().unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_write_to_nonexistent_fails() {
        let store = make_store();
        let result = store.write_file("z1", "nope", b"data");
        assert!(matches!(result, Err(StoreError::NotFound)));
    }

    #[test]
    fn test_circular_file_data_length() {
        let file = WaveFile {
            zoneid: "z1".to_string(),
            name: "circ".to_string(),
            size: 200,
            createdts: 0,
            modts: 0,
            opts: FileOpts {
                maxsize: 100,
                circular: true,
                ..Default::default()
            },
            meta: FileMeta::new(),
        };
        assert_eq!(file.data_length(), 100);
        assert_eq!(file.data_start_idx(), 100);
    }

    #[test]
    fn test_circular_file_under_max() {
        let file = WaveFile {
            zoneid: "z1".to_string(),
            name: "circ".to_string(),
            size: 50,
            createdts: 0,
            modts: 0,
            opts: FileOpts {
                maxsize: 100,
                circular: true,
                ..Default::default()
            },
            meta: FileMeta::new(),
        };
        assert_eq!(file.data_length(), 50);
        assert_eq!(file.data_start_idx(), 0);
    }

    #[test]
    fn test_file_meta_with_opts() {
        let store = make_store();
        let mut meta = FileMeta::new();
        meta.insert("custom".into(), serde_json::json!("value"));

        let opts = FileOpts {
            maxsize: 1024,
            circular: true,
            ..Default::default()
        };

        store.make_file("z1", "f1", meta, opts).unwrap();
        let file = store.stat("z1", "f1").unwrap().unwrap();
        assert_eq!(file.opts.maxsize, 1024);
        assert!(file.opts.circular);
        assert_eq!(file.meta.get("custom").unwrap(), "value");
    }

    #[test]
    fn test_flush_cache_no_dirty() {
        let store = make_store();
        let (files, parts) = store.flush_cache().unwrap();
        assert_eq!(files, 0);
        assert_eq!(parts, 0);
    }
}
