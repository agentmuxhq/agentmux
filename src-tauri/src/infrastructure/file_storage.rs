// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! FileStore adapter — implements the domain FileStore trait using the
//! concrete SQLite-backed FileStore backend.

use crate::backend::storage::filestore::FileStore as BackendFileStore;
use crate::domain::traits::{FileStore, RepoResult, RepositoryError};

/// Adapter wrapping the backend FileStore to implement the domain FileStore trait.
pub struct FileStoreAdapter {
    store: BackendFileStore,
}

impl FileStoreAdapter {
    pub fn new(store: BackendFileStore) -> Self {
        Self { store }
    }

    /// Access the underlying backend FileStore.
    pub fn inner(&self) -> &BackendFileStore {
        &self.store
    }
}

impl FileStore for FileStoreAdapter {
    fn read_file(&self, zone_id: &str, name: &str) -> RepoResult<Vec<u8>> {
        self.store
            .read_file(zone_id, name)
            .map_err(|e| RepositoryError::Storage(e.to_string()))?
            .ok_or_else(|| {
                RepositoryError::NotFound(format!("file not found: {}:{}", zone_id, name))
            })
    }

    fn write_file(&self, zone_id: &str, name: &str, data: &[u8]) -> RepoResult<()> {
        self.store
            .write_file(zone_id, name, data)
            .map_err(|e| RepositoryError::Storage(e.to_string()))
    }

    fn delete_file(&self, zone_id: &str, name: &str) -> RepoResult<()> {
        self.store
            .delete_file(zone_id, name)
            .map_err(|e| RepositoryError::Storage(e.to_string()))
    }

    fn list_files(&self, zone_id: &str) -> RepoResult<Vec<String>> {
        self.store
            .list_files(zone_id)
            .map(|files| files.into_iter().map(|f| f.name).collect())
            .map_err(|e| RepositoryError::Storage(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::storage::filestore::{FileMeta, FileOpts};

    fn make_adapter() -> FileStoreAdapter {
        let store = BackendFileStore::open_in_memory().unwrap();
        FileStoreAdapter::new(store)
    }

    #[test]
    fn test_write_and_read_file() {
        let adapter = make_adapter();
        let zone = "test-zone";
        let name = "test.txt";

        // Must create file first via backend
        adapter
            .inner()
            .make_file(zone, name, FileMeta::new(), FileOpts::default())
            .unwrap();

        adapter.write_file(zone, name, b"hello world").unwrap();
        let data = adapter.read_file(zone, name).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn test_delete_file() {
        let adapter = make_adapter();
        let zone = "test-zone";
        let name = "delete-me.txt";

        adapter
            .inner()
            .make_file(zone, name, FileMeta::new(), FileOpts::default())
            .unwrap();
        adapter.write_file(zone, name, b"data").unwrap();

        adapter.delete_file(zone, name).unwrap();
        assert!(adapter.read_file(zone, name).is_err());
    }

    #[test]
    fn test_list_files() {
        let adapter = make_adapter();
        let zone = "zone-list";

        adapter
            .inner()
            .make_file(zone, "a.txt", FileMeta::new(), FileOpts::default())
            .unwrap();
        adapter
            .inner()
            .make_file(zone, "b.txt", FileMeta::new(), FileOpts::default())
            .unwrap();

        let files = adapter.list_files(zone).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"a.txt".to_string()));
        assert!(files.contains(&"b.txt".to_string()));
    }

    #[test]
    fn test_read_nonexistent() {
        let adapter = make_adapter();
        let result = adapter.read_file("no-zone", "no-file");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }
}
