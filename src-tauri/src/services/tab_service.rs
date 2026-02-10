// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Tab service — business logic for tab and block management.

use crate::domain::entities::{LayoutState, Tab};
use crate::domain::traits::{ObjectStore, RepoResult, RepositoryError};
use crate::domain::value_objects::*;
use std::sync::Arc;
use uuid::Uuid;

pub struct TabService {
    store: Arc<dyn ObjectStore>,
}

impl TabService {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    /// Get a tab by ID.
    pub fn get_tab(&self, oid: &str) -> RepoResult<Tab> {
        let json = self.store.get_object_json(OTYPE_TAB, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Create a new tab with empty layout.
    pub fn create_tab(&self, name: &str) -> RepoResult<Tab> {
        let tab_id = Uuid::new_v4().to_string();
        let layout_id = Uuid::new_v4().to_string();

        // Create empty layout
        let layout = LayoutState {
            oid: layout_id.clone(),
            version: 1,
            ..Default::default()
        };
        let layout_json = serde_json::to_value(&layout)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(OTYPE_LAYOUT, &layout.oid, &layout_json)?;

        // Create tab
        let tab = Tab {
            oid: tab_id,
            version: 1,
            name: name.to_string(),
            layoutstate: layout_id,
            blockids: Vec::new(),
            meta: MetaMapType::new(),
        };
        let tab_json = serde_json::to_value(&tab)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(OTYPE_TAB, &tab.oid, &tab_json)?;

        Ok(tab)
    }

    /// Add a block to a tab.
    pub fn add_block(&self, tab_id: &str, block_id: &str) -> RepoResult<()> {
        let mut tab = self.get_tab(tab_id)?;

        if tab.blockids.contains(&block_id.to_string()) {
            return Ok(()); // Already present
        }

        tab.blockids.push(block_id.to_string());
        tab.version += 1;

        let json = serde_json::to_value(&tab)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store.set_object_json(OTYPE_TAB, &tab.oid, &json)?;

        Ok(())
    }

    /// Remove a block from a tab.
    pub fn remove_block(&self, tab_id: &str, block_id: &str) -> RepoResult<bool> {
        let mut tab = self.get_tab(tab_id)?;

        let before_len = tab.blockids.len();
        tab.blockids.retain(|id| id != block_id);

        if tab.blockids.len() == before_len {
            return Ok(false); // Not found
        }

        tab.version += 1;
        let json = serde_json::to_value(&tab)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store.set_object_json(OTYPE_TAB, &tab.oid, &json)?;

        Ok(true)
    }

    /// Delete a tab and its layout.
    pub fn delete_tab(&self, oid: &str) -> RepoResult<()> {
        let tab = self.get_tab(oid)?;

        // Delete layout
        if !tab.layoutstate.is_empty() {
            let _ = self.store.delete_object(OTYPE_LAYOUT, &tab.layoutstate);
        }

        self.store.delete_object(OTYPE_TAB, oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::traits::mock::MockObjectStore;

    #[test]
    fn test_create_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("My Tab").unwrap();
        assert_eq!(tab.name, "My Tab");
        assert!(!tab.layoutstate.is_empty());
        assert!(tab.blockids.is_empty());

        // Verify stored
        let loaded = service.get_tab(&tab.oid).unwrap();
        assert_eq!(loaded.name, "My Tab");
    }

    #[test]
    fn test_add_and_remove_block() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("Test").unwrap();

        // Add block
        service.add_block(&tab.oid, "block-1").unwrap();
        let loaded = service.get_tab(&tab.oid).unwrap();
        assert_eq!(loaded.blockids, vec!["block-1"]);
        assert_eq!(loaded.version, 2);

        // Add same block again (no-op)
        service.add_block(&tab.oid, "block-1").unwrap();
        let loaded = service.get_tab(&tab.oid).unwrap();
        assert_eq!(loaded.blockids.len(), 1);

        // Remove block
        let removed = service.remove_block(&tab.oid, "block-1").unwrap();
        assert!(removed);
        let loaded = service.get_tab(&tab.oid).unwrap();
        assert!(loaded.blockids.is_empty());
    }

    #[test]
    fn test_delete_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("ToDelete").unwrap();
        let tab_id = tab.oid.clone();

        service.delete_tab(&tab_id).unwrap();
        assert!(service.get_tab(&tab_id).is_err());
    }

    #[test]
    fn test_delete_tab_cascades_layout() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store.clone());

        let tab = service.create_tab("CascadeTest").unwrap();
        let layout_id = tab.layoutstate.clone();
        assert!(!layout_id.is_empty());

        // Layout should exist
        assert!(store.get_object_json(OTYPE_LAYOUT, &layout_id).is_ok());

        service.delete_tab(&tab.oid).unwrap();

        // Layout should be gone
        assert!(store.get_object_json(OTYPE_LAYOUT, &layout_id).is_err());
    }

    #[test]
    fn test_add_multiple_blocks() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("Multi").unwrap();
        service.add_block(&tab.oid, "block-1").unwrap();
        service.add_block(&tab.oid, "block-2").unwrap();
        service.add_block(&tab.oid, "block-3").unwrap();

        let loaded = service.get_tab(&tab.oid).unwrap();
        assert_eq!(loaded.blockids.len(), 3);
        assert_eq!(loaded.version, 4); // 1 initial + 3 adds
    }

    #[test]
    fn test_remove_nonexistent_block() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("Test").unwrap();
        let removed = service.remove_block(&tab.oid, "nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_create_tab_with_empty_name() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let tab = service.create_tab("").unwrap();
        assert!(tab.name.is_empty());
        assert!(!tab.oid.is_empty());
        assert!(!tab.layoutstate.is_empty());
    }

    #[test]
    fn test_get_nonexistent_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let result = service.get_tab("nonexistent");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_add_block_to_nonexistent_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let result = service.add_block("nonexistent", "block-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_block_from_nonexistent_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = TabService::new(store);

        let result = service.remove_block("nonexistent", "block-1");
        assert!(result.is_err());
    }
}
