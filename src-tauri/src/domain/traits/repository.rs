// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Repository trait definitions — contracts that storage implementations must fulfill.

use crate::domain::entities::{Block, Client, LayoutState, Tab, Window, Workspace};

/// Unified error type for repository operations.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("entity not found: {0}")]
    NotFound(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("constraint violation: {0}")]
    Constraint(String),
}

pub type RepoResult<T> = std::result::Result<T, RepositoryError>;

/// Generic object store — can get/set/delete any WaveObj by otype+oid.
pub trait ObjectStore: Send + Sync {
    fn get_object_json(&self, otype: &str, oid: &str) -> RepoResult<serde_json::Value>;
    fn set_object_json(&self, otype: &str, oid: &str, data: &serde_json::Value) -> RepoResult<()>;
    fn delete_object(&self, otype: &str, oid: &str) -> RepoResult<()>;
}

/// Typed repository for Client entities.
pub trait ClientRepository: Send + Sync {
    fn get_client(&self) -> RepoResult<Client>;
    fn save_client(&self, client: &Client) -> RepoResult<()>;
}

/// Typed repository for Window entities.
pub trait WindowRepository: Send + Sync {
    fn get_window(&self, oid: &str) -> RepoResult<Window>;
    fn save_window(&self, window: &Window) -> RepoResult<()>;
    fn delete_window(&self, oid: &str) -> RepoResult<()>;
}

/// Typed repository for Workspace entities.
pub trait WorkspaceRepository: Send + Sync {
    fn get_workspace(&self, oid: &str) -> RepoResult<Workspace>;
    fn save_workspace(&self, workspace: &Workspace) -> RepoResult<()>;
    fn delete_workspace(&self, oid: &str) -> RepoResult<()>;
    fn list_workspaces(&self) -> RepoResult<Vec<Workspace>>;
}

/// Typed repository for Tab entities.
pub trait TabRepository: Send + Sync {
    fn get_tab(&self, oid: &str) -> RepoResult<Tab>;
    fn save_tab(&self, tab: &Tab) -> RepoResult<()>;
    fn delete_tab(&self, oid: &str) -> RepoResult<()>;
}

/// Typed repository for Block entities.
pub trait BlockRepository: Send + Sync {
    fn get_block(&self, oid: &str) -> RepoResult<Block>;
    fn save_block(&self, block: &Block) -> RepoResult<()>;
    fn delete_block(&self, oid: &str) -> RepoResult<()>;
}

/// Typed repository for LayoutState entities.
pub trait LayoutRepository: Send + Sync {
    fn get_layout(&self, oid: &str) -> RepoResult<LayoutState>;
    fn save_layout(&self, layout: &LayoutState) -> RepoResult<()>;
    fn delete_layout(&self, oid: &str) -> RepoResult<()>;
}

/// File content store — separate from the object store.
pub trait FileStore: Send + Sync {
    fn read_file(&self, zone_id: &str, name: &str) -> RepoResult<Vec<u8>>;
    fn write_file(&self, zone_id: &str, name: &str, data: &[u8]) -> RepoResult<()>;
    fn delete_file(&self, zone_id: &str, name: &str) -> RepoResult<()>;
    fn list_files(&self, zone_id: &str) -> RepoResult<Vec<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_error_display() {
        let err = RepositoryError::NotFound("client:abc".to_string());
        assert_eq!(err.to_string(), "entity not found: client:abc");

        let err = RepositoryError::Storage("disk full".to_string());
        assert_eq!(err.to_string(), "storage error: disk full");
    }
}
