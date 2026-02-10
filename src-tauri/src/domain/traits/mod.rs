// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Repository traits: contracts for storage implementations.

mod repository;

pub use repository::*;

/// In-memory mock implementation of `ObjectStore` for use in tests.
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// A simple in-memory store implementing `ObjectStore`.
    /// Keys are `"otype:oid"` strings mapped to JSON values.
    pub struct MockObjectStore {
        data: Mutex<HashMap<String, serde_json::Value>>,
    }

    impl MockObjectStore {
        pub fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
            }
        }

        fn key(otype: &str, oid: &str) -> String {
            format!("{otype}:{oid}")
        }
    }

    impl ObjectStore for MockObjectStore {
        fn get_object_json(&self, otype: &str, oid: &str) -> RepoResult<serde_json::Value> {
            let key = Self::key(otype, oid);
            self.data
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .ok_or_else(|| RepositoryError::NotFound(key))
        }

        fn set_object_json(
            &self,
            otype: &str,
            oid: &str,
            data: &serde_json::Value,
        ) -> RepoResult<()> {
            let key = Self::key(otype, oid);
            self.data.lock().unwrap().insert(key, data.clone());
            Ok(())
        }

        fn delete_object(&self, otype: &str, oid: &str) -> RepoResult<()> {
            let key = Self::key(otype, oid);
            self.data
                .lock()
                .unwrap()
                .remove(&key)
                .ok_or_else(|| RepositoryError::NotFound(key))?;
            Ok(())
        }
    }
}
