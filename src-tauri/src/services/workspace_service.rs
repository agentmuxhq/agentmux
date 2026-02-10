// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Workspace service — business logic for workspace and tab management.

use crate::domain::entities::{LayoutState, Tab, Workspace};
use crate::domain::traits::{ObjectStore, RepoResult, RepositoryError};
use crate::domain::value_objects::*;
use std::sync::Arc;
use uuid::Uuid;

pub struct WorkspaceService {
    store: Arc<dyn ObjectStore>,
}

impl WorkspaceService {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    /// Get a workspace by ID.
    pub fn get_workspace(&self, oid: &str) -> RepoResult<Workspace> {
        let json = self.store.get_object_json(OTYPE_WORKSPACE, oid)?;
        serde_json::from_value(json).map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    /// Create a new workspace with a default tab.
    pub fn create_workspace(&self, name: &str, icon: &str, color: &str) -> RepoResult<Workspace> {
        let workspace_id = Uuid::new_v4().to_string();
        let tab_id = Uuid::new_v4().to_string();
        let layout_id = Uuid::new_v4().to_string();

        // Create empty layout for the default tab
        let layout = LayoutState {
            oid: layout_id.clone(),
            version: 1,
            ..Default::default()
        };
        let layout_json = serde_json::to_value(&layout)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(OTYPE_LAYOUT, &layout.oid, &layout_json)?;

        // Create default tab
        let tab = Tab {
            oid: tab_id.clone(),
            version: 1,
            name: String::new(),
            layoutstate: layout_id,
            blockids: Vec::new(),
            meta: MetaMapType::new(),
        };
        let tab_json = serde_json::to_value(&tab)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store.set_object_json(OTYPE_TAB, &tab.oid, &tab_json)?;

        // Create workspace
        let workspace = Workspace {
            oid: workspace_id,
            version: 1,
            name: name.to_string(),
            icon: icon.to_string(),
            color: color.to_string(),
            tabids: vec![tab_id.clone()],
            pinnedtabids: Vec::new(),
            activetabid: tab_id,
            meta: MetaMapType::new(),
        };
        let ws_json = serde_json::to_value(&workspace)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(OTYPE_WORKSPACE, &workspace.oid, &ws_json)?;

        Ok(workspace)
    }

    /// Set the active tab for a workspace.
    pub fn set_active_tab(&self, workspace_id: &str, tab_id: &str) -> RepoResult<()> {
        let mut workspace = self.get_workspace(workspace_id)?;

        if !workspace.tabids.contains(&tab_id.to_string()) {
            return Err(RepositoryError::Constraint(format!(
                "tab {} not in workspace {}",
                tab_id, workspace_id
            )));
        }

        workspace.activetabid = tab_id.to_string();
        workspace.version += 1;

        let json = serde_json::to_value(&workspace)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
        self.store
            .set_object_json(OTYPE_WORKSPACE, &workspace.oid, &json)?;

        Ok(())
    }

    /// Delete a workspace and all its tabs.
    pub fn delete_workspace(&self, oid: &str) -> RepoResult<()> {
        let workspace = self.get_workspace(oid)?;

        // Delete all tabs and their layouts
        for tab_id in &workspace.tabids {
            // Try to load tab to find its layout
            if let Ok(tab_json) = self.store.get_object_json(OTYPE_TAB, tab_id) {
                if let Ok(tab) = serde_json::from_value::<Tab>(tab_json) {
                    if !tab.layoutstate.is_empty() {
                        let _ = self.store.delete_object(OTYPE_LAYOUT, &tab.layoutstate);
                    }
                }
            }
            let _ = self.store.delete_object(OTYPE_TAB, tab_id);
        }

        self.store.delete_object(OTYPE_WORKSPACE, oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::traits::mock::MockObjectStore;

    #[test]
    fn test_create_workspace() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store.clone());

        let ws = service
            .create_workspace("My Workspace", "terminal", "#58C142")
            .unwrap();

        assert_eq!(ws.name, "My Workspace");
        assert_eq!(ws.icon, "terminal");
        assert_eq!(ws.color, "#58C142");
        assert_eq!(ws.tabids.len(), 1);
        assert_eq!(ws.activetabid, ws.tabids[0]);

        // Verify workspace is stored
        let loaded = service.get_workspace(&ws.oid).unwrap();
        assert_eq!(loaded.name, "My Workspace");

        // Verify the default tab's layout was created in the store
        let tab_json = store
            .get_object_json(OTYPE_TAB, &ws.tabids[0])
            .unwrap();
        let tab: Tab = serde_json::from_value(tab_json).unwrap();
        assert!(!tab.layoutstate.is_empty());
        let layout_json = store
            .get_object_json(OTYPE_LAYOUT, &tab.layoutstate)
            .unwrap();
        let layout: LayoutState = serde_json::from_value(layout_json).unwrap();
        assert_eq!(layout.version, 1);
    }

    #[test]
    fn test_set_active_tab() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let ws = service
            .create_workspace("Test", "icon", "#000")
            .unwrap();
        let tab_id = &ws.tabids[0];

        // Setting active to existing tab should work
        service.set_active_tab(&ws.oid, tab_id).unwrap();

        // Setting active to non-existent tab should fail
        let result = service.set_active_tab(&ws.oid, "nonexistent-tab");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_workspace() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let ws = service
            .create_workspace("ToDelete", "icon", "#000")
            .unwrap();
        let ws_id = ws.oid.clone();

        service.delete_workspace(&ws_id).unwrap();

        // Workspace should be gone
        assert!(service.get_workspace(&ws_id).is_err());
    }

    #[test]
    fn test_delete_workspace_cascades_tabs_and_layouts() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store.clone());

        let ws = service
            .create_workspace("Cascade", "icon", "#000")
            .unwrap();
        let tab_id = ws.tabids[0].clone();

        // Get the tab's layout ID before deletion
        let tab_json = store.get_object_json(OTYPE_TAB, &tab_id).unwrap();
        let tab: Tab = serde_json::from_value(tab_json).unwrap();
        let layout_id = tab.layoutstate.clone();
        assert!(!layout_id.is_empty());

        // Verify tab and layout exist
        assert!(store.get_object_json(OTYPE_TAB, &tab_id).is_ok());
        assert!(store.get_object_json(OTYPE_LAYOUT, &layout_id).is_ok());

        // Delete workspace
        service.delete_workspace(&ws.oid).unwrap();

        // Tab and layout should both be gone
        assert!(store.get_object_json(OTYPE_TAB, &tab_id).is_err());
        assert!(store.get_object_json(OTYPE_LAYOUT, &layout_id).is_err());
    }

    #[test]
    fn test_set_active_tab_updates_stored() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let ws = service
            .create_workspace("Test", "icon", "#000")
            .unwrap();
        let tab_id = &ws.tabids[0];

        service.set_active_tab(&ws.oid, tab_id).unwrap();

        let loaded = service.get_workspace(&ws.oid).unwrap();
        assert_eq!(loaded.activetabid, *tab_id);
        assert_eq!(loaded.version, 2); // version incremented
    }

    #[test]
    fn test_create_multiple_workspaces() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let ws1 = service.create_workspace("WS1", "terminal", "#111").unwrap();
        let ws2 = service.create_workspace("WS2", "code", "#222").unwrap();

        assert_ne!(ws1.oid, ws2.oid);
        assert_ne!(ws1.tabids[0], ws2.tabids[0]);

        let loaded1 = service.get_workspace(&ws1.oid).unwrap();
        let loaded2 = service.get_workspace(&ws2.oid).unwrap();
        assert_eq!(loaded1.name, "WS1");
        assert_eq!(loaded2.name, "WS2");
    }

    #[test]
    fn test_delete_nonexistent_workspace() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let result = service.delete_workspace("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent_workspace() {
        let store = Arc::new(MockObjectStore::new());
        let service = WorkspaceService::new(store);

        let result = service.get_workspace("nonexistent");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }
}
