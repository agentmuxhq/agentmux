// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Generic object service — get/set/delete any WaveObj by otype+oid.
//! This is the service-layer equivalent of the current wcore module.

use crate::domain::entities::*;
use crate::domain::traits::{ObjectStore, RepoResult, RepositoryError};
use crate::domain::value_objects::*;
use std::sync::Arc;

pub struct ObjectService {
    store: Arc<dyn ObjectStore>,
}

impl ObjectService {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    /// Get any WaveObj as JSON by otype and oid.
    pub fn get_object(&self, otype: &str, oid: &str) -> RepoResult<serde_json::Value> {
        if !VALID_OTYPES.contains(&otype) {
            return Err(RepositoryError::NotFound(format!(
                "unknown otype: {otype}"
            )));
        }
        self.store.get_object_json(otype, oid)
    }

    /// Get a typed Client entity.
    pub fn get_client(&self, oid: &str) -> RepoResult<Client> {
        let json = self.store.get_object_json(OTYPE_CLIENT, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Get a typed Window entity.
    pub fn get_window(&self, oid: &str) -> RepoResult<Window> {
        let json = self.store.get_object_json(OTYPE_WINDOW, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Get a typed Workspace entity.
    pub fn get_workspace(&self, oid: &str) -> RepoResult<Workspace> {
        let json = self.store.get_object_json(OTYPE_WORKSPACE, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Get a typed Tab entity.
    pub fn get_tab(&self, oid: &str) -> RepoResult<Tab> {
        let json = self.store.get_object_json(OTYPE_TAB, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Get a typed Block entity.
    pub fn get_block(&self, oid: &str) -> RepoResult<Block> {
        let json = self.store.get_object_json(OTYPE_BLOCK, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Get a typed LayoutState entity.
    pub fn get_layout(&self, oid: &str) -> RepoResult<LayoutState> {
        let json = self.store.get_object_json(OTYPE_LAYOUT, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Save any WaveObj to storage.
    pub fn save_object<T: WaveObj>(&self, obj: &T) -> RepoResult<()> {
        let json = serde_json::to_value(obj)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(T::get_otype(), obj.get_oid(), &json)
    }

    /// Delete any WaveObj from storage.
    pub fn delete_object(&self, otype: &str, oid: &str) -> RepoResult<()> {
        self.store.delete_object(otype, oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockObjectStore {
        objects: Mutex<HashMap<String, serde_json::Value>>,
    }

    impl MockObjectStore {
        fn new() -> Self {
            Self {
                objects: Mutex::new(HashMap::new()),
            }
        }

        fn key(otype: &str, oid: &str) -> String {
            format!("{otype}:{oid}")
        }
    }

    impl ObjectStore for MockObjectStore {
        fn get_object_json(&self, otype: &str, oid: &str) -> RepoResult<serde_json::Value> {
            let key = Self::key(otype, oid);
            self.objects
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
            self.objects.lock().unwrap().insert(key, data.clone());
            Ok(())
        }

        fn delete_object(&self, otype: &str, oid: &str) -> RepoResult<()> {
            let key = Self::key(otype, oid);
            self.objects
                .lock()
                .unwrap()
                .remove(&key)
                .ok_or_else(|| RepositoryError::NotFound(key))?;
            Ok(())
        }
    }

    #[test]
    fn test_save_and_get_client() {
        let store = Arc::new(MockObjectStore::new());
        let service = ObjectService::new(store);

        let client = Client {
            oid: "test-client".to_string(),
            version: 1,
            windowids: vec!["w1".to_string()],
            ..Default::default()
        };

        service.save_object(&client).unwrap();
        let loaded = service.get_client("test-client").unwrap();
        assert_eq!(loaded.oid, "test-client");
        assert_eq!(loaded.windowids, vec!["w1"]);
    }

    #[test]
    fn test_save_and_get_workspace() {
        let store = Arc::new(MockObjectStore::new());
        let service = ObjectService::new(store);

        let ws = Workspace {
            oid: "ws-1".to_string(),
            version: 1,
            name: "Test".to_string(),
            tabids: vec!["t1".to_string()],
            activetabid: "t1".to_string(),
            ..Default::default()
        };

        service.save_object(&ws).unwrap();
        let loaded = service.get_workspace("ws-1").unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.tabids.len(), 1);
    }

    #[test]
    fn test_get_nonexistent() {
        let store = Arc::new(MockObjectStore::new());
        let service = ObjectService::new(store);
        let result = service.get_client("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_object() {
        let store = Arc::new(MockObjectStore::new());
        let service = ObjectService::new(store);

        let tab = Tab {
            oid: "tab-1".to_string(),
            version: 1,
            name: "Shell".to_string(),
            ..Default::default()
        };

        service.save_object(&tab).unwrap();
        service.delete_object(OTYPE_TAB, "tab-1").unwrap();
        assert!(service.get_tab("tab-1").is_err());
    }

    #[test]
    fn test_get_object_unknown_otype() {
        let store = Arc::new(MockObjectStore::new());
        let service = ObjectService::new(store);
        let result = service.get_object("foobar", "some-id");
        assert!(result.is_err());
    }
}
