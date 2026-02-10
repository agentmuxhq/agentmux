// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! WaveStore adapter — implements domain repository traits using SQLite storage.
//!
//! This bridges the domain `ObjectStore` and typed repository traits to the
//! concrete `WaveStore` backend.

use crate::backend::storage::error::StoreError;
use crate::backend::storage::wstore::WaveStore;
use crate::domain::entities::*;
use crate::domain::traits::*;
use crate::domain::value_objects::*;

/// Adapter that implements domain `ObjectStore` over `WaveStore`.
pub struct WaveStoreAdapter {
    store: WaveStore,
}

impl WaveStoreAdapter {
    pub fn new(store: WaveStore) -> Self {
        Self { store }
    }

    /// Access the underlying WaveStore (for callers that need direct access).
    pub fn inner(&self) -> &WaveStore {
        &self.store
    }
}

// ---- Error conversion ----

fn store_err_to_repo(e: StoreError) -> RepositoryError {
    match e {
        StoreError::NotFound => RepositoryError::NotFound("not found".into()),
        StoreError::AlreadyExists => {
            RepositoryError::Constraint("already exists".into())
        }
        StoreError::EmptyOID => RepositoryError::Constraint("empty OID".into()),
        StoreError::VersionMismatch { expected, actual } => {
            RepositoryError::Constraint(format!(
                "version mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        StoreError::Json(e) => RepositoryError::Serialization(e.to_string()),
        StoreError::Sqlite(e) => RepositoryError::Storage(e.to_string()),
        StoreError::Other(s) => RepositoryError::Storage(s),
    }
}

// ---- Generic ObjectStore ----

impl ObjectStore for WaveStoreAdapter {
    fn get_object_json(&self, otype: &str, oid: &str) -> RepoResult<serde_json::Value> {
        let json = match otype {
            OTYPE_CLIENT => self
                .store
                .must_get::<Client>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            OTYPE_WINDOW => self
                .store
                .must_get::<Window>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            OTYPE_WORKSPACE => self
                .store
                .must_get::<Workspace>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            OTYPE_TAB => self
                .store
                .must_get::<Tab>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            OTYPE_LAYOUT => self
                .store
                .must_get::<LayoutState>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            OTYPE_BLOCK => self
                .store
                .must_get::<Block>(oid)
                .map(|o| serde_json::to_value(&o).unwrap_or_default()),
            _ => return Err(RepositoryError::NotFound(format!("unknown otype: {}", otype))),
        };
        json.map_err(store_err_to_repo)
    }

    fn set_object_json(
        &self,
        otype: &str,
        oid: &str,
        data: &serde_json::Value,
    ) -> RepoResult<()> {
        match otype {
            OTYPE_CLIENT => {
                let mut obj: Client =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            OTYPE_WINDOW => {
                let mut obj: Window =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            OTYPE_WORKSPACE => {
                let mut obj: Workspace =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            OTYPE_TAB => {
                let mut obj: Tab =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            OTYPE_LAYOUT => {
                let mut obj: LayoutState =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            OTYPE_BLOCK => {
                let mut obj: Block =
                    serde_json::from_value(data.clone()).map_err(|e| RepositoryError::Serialization(e.to_string()))?;
                if obj.oid.is_empty() {
                    obj.oid = oid.to_string();
                }
                upsert(&self.store, &mut obj)
            }
            _ => Err(RepositoryError::NotFound(format!("unknown otype: {}", otype))),
        }
    }

    fn delete_object(&self, otype: &str, oid: &str) -> RepoResult<()> {
        self.store
            .delete_by_otype(otype, oid)
            .map_err(store_err_to_repo)
    }
}

/// Upsert: try update first, if not found then insert.
/// WaveStore.update() returns Sqlite(QueryReturnedNoRows) when the row doesn't
/// exist, so we catch both NotFound and that specific Sqlite error.
fn upsert<T: WaveObj>(store: &WaveStore, obj: &mut T) -> RepoResult<()> {
    match store.update(obj) {
        Ok(_) => Ok(()),
        Err(StoreError::NotFound) => store.insert(obj).map_err(store_err_to_repo),
        Err(StoreError::Sqlite(ref e))
            if e.to_string().contains("Query returned no rows") =>
        {
            store.insert(obj).map_err(store_err_to_repo)
        }
        Err(e) => Err(store_err_to_repo(e)),
    }
}

// ---- Typed repositories ----

impl ClientRepository for WaveStoreAdapter {
    fn get_client(&self) -> RepoResult<Client> {
        // Client is a singleton — get_all and take first
        let clients = self.store.get_all::<Client>().map_err(store_err_to_repo)?;
        clients
            .into_iter()
            .next()
            .ok_or_else(|| RepositoryError::NotFound("no client found".into()))
    }

    fn save_client(&self, client: &Client) -> RepoResult<()> {
        let mut c = client.clone();
        upsert(&self.store, &mut c)
    }
}

impl WindowRepository for WaveStoreAdapter {
    fn get_window(&self, oid: &str) -> RepoResult<Window> {
        self.store
            .must_get::<Window>(oid)
            .map_err(store_err_to_repo)
    }

    fn save_window(&self, window: &Window) -> RepoResult<()> {
        let mut w = window.clone();
        upsert(&self.store, &mut w)
    }

    fn delete_window(&self, oid: &str) -> RepoResult<()> {
        self.store
            .delete::<Window>(oid)
            .map_err(store_err_to_repo)
    }
}

impl WorkspaceRepository for WaveStoreAdapter {
    fn get_workspace(&self, oid: &str) -> RepoResult<Workspace> {
        self.store
            .must_get::<Workspace>(oid)
            .map_err(store_err_to_repo)
    }

    fn save_workspace(&self, workspace: &Workspace) -> RepoResult<()> {
        let mut ws = workspace.clone();
        upsert(&self.store, &mut ws)
    }

    fn delete_workspace(&self, oid: &str) -> RepoResult<()> {
        self.store
            .delete::<Workspace>(oid)
            .map_err(store_err_to_repo)
    }

    fn list_workspaces(&self) -> RepoResult<Vec<Workspace>> {
        self.store
            .get_all::<Workspace>()
            .map_err(store_err_to_repo)
    }
}

impl TabRepository for WaveStoreAdapter {
    fn get_tab(&self, oid: &str) -> RepoResult<Tab> {
        self.store.must_get::<Tab>(oid).map_err(store_err_to_repo)
    }

    fn save_tab(&self, tab: &Tab) -> RepoResult<()> {
        let mut t = tab.clone();
        upsert(&self.store, &mut t)
    }

    fn delete_tab(&self, oid: &str) -> RepoResult<()> {
        self.store.delete::<Tab>(oid).map_err(store_err_to_repo)
    }
}

impl BlockRepository for WaveStoreAdapter {
    fn get_block(&self, oid: &str) -> RepoResult<Block> {
        self.store.must_get::<Block>(oid).map_err(store_err_to_repo)
    }

    fn save_block(&self, block: &Block) -> RepoResult<()> {
        let mut b = block.clone();
        upsert(&self.store, &mut b)
    }

    fn delete_block(&self, oid: &str) -> RepoResult<()> {
        self.store.delete::<Block>(oid).map_err(store_err_to_repo)
    }
}

impl LayoutRepository for WaveStoreAdapter {
    fn get_layout(&self, oid: &str) -> RepoResult<LayoutState> {
        self.store
            .must_get::<LayoutState>(oid)
            .map_err(store_err_to_repo)
    }

    fn save_layout(&self, layout: &LayoutState) -> RepoResult<()> {
        let mut l = layout.clone();
        upsert(&self.store, &mut l)
    }

    fn delete_layout(&self, oid: &str) -> RepoResult<()> {
        self.store
            .delete::<LayoutState>(oid)
            .map_err(store_err_to_repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_adapter() -> WaveStoreAdapter {
        let store = WaveStore::open_in_memory().unwrap();
        WaveStoreAdapter::new(store)
    }

    #[test]
    fn test_client_repository() {
        let adapter = make_adapter();

        // No client initially
        assert!(adapter.get_client().is_err());

        // Save and retrieve
        let client = Client {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            windowids: vec!["win-1".into()],
            meta: MetaMapType::new(),
            tosagreed: 0,
            hasoldhistory: false,
            tempoid: String::new(),
        };
        adapter.save_client(&client).unwrap();

        let loaded = adapter.get_client().unwrap();
        assert_eq!(loaded.oid, client.oid);
        assert_eq!(loaded.windowids, vec!["win-1"]);
    }

    #[test]
    fn test_workspace_repository() {
        let adapter = make_adapter();

        let ws = Workspace {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            name: "Test WS".into(),
            icon: "terminal".into(),
            color: "#58C142".into(),
            tabids: vec![],
            pinnedtabids: vec![],
            activetabid: String::new(),
            meta: MetaMapType::new(),
        };

        adapter.save_workspace(&ws).unwrap();

        let loaded = adapter.get_workspace(&ws.oid).unwrap();
        assert_eq!(loaded.name, "Test WS");

        let all = adapter.list_workspaces().unwrap();
        assert_eq!(all.len(), 1);

        adapter.delete_workspace(&ws.oid).unwrap();
        assert!(adapter.get_workspace(&ws.oid).is_err());
    }

    #[test]
    fn test_object_store_generic() {
        let adapter = make_adapter();

        let tab = Tab {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            name: "My Tab".into(),
            layoutstate: String::new(),
            blockids: vec![],
            meta: MetaMapType::new(),
        };

        let tab_json = serde_json::to_value(&tab).unwrap();
        adapter
            .set_object_json(OTYPE_TAB, &tab.oid, &tab_json)
            .unwrap();

        let loaded = adapter.get_object_json(OTYPE_TAB, &tab.oid).unwrap();
        assert_eq!(loaded.get("name").unwrap().as_str().unwrap(), "My Tab");

        adapter.delete_object(OTYPE_TAB, &tab.oid).unwrap();
        assert!(adapter.get_object_json(OTYPE_TAB, &tab.oid).is_err());
    }

    #[test]
    fn test_block_repository() {
        let adapter = make_adapter();

        let block = Block {
            oid: uuid::Uuid::new_v4().to_string(),
            parentoref: String::new(),
            version: 1,
            runtimeopts: None,
            stickers: None,
            meta: MetaMapType::new(),
            subblockids: None,
        };

        adapter.save_block(&block).unwrap();
        let loaded = adapter.get_block(&block.oid).unwrap();
        assert_eq!(loaded.oid, block.oid);
    }

    #[test]
    fn test_error_conversion() {
        let adapter = make_adapter();

        let result = adapter.get_workspace("nonexistent");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_window_repository() {
        let adapter = make_adapter();

        let window = Window {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            workspaceid: "ws-1".into(),
            ..Default::default()
        };

        adapter.save_window(&window).unwrap();
        let loaded = adapter.get_window(&window.oid).unwrap();
        assert_eq!(loaded.workspaceid, "ws-1");

        adapter.delete_window(&window.oid).unwrap();
        assert!(adapter.get_window(&window.oid).is_err());
    }

    #[test]
    fn test_tab_repository() {
        let adapter = make_adapter();

        let tab = Tab {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            name: "Shell".into(),
            layoutstate: "ls-1".into(),
            blockids: vec!["b1".into()],
            meta: MetaMapType::new(),
        };

        adapter.save_tab(&tab).unwrap();
        let loaded = adapter.get_tab(&tab.oid).unwrap();
        assert_eq!(loaded.name, "Shell");
        assert_eq!(loaded.blockids, vec!["b1"]);

        adapter.delete_tab(&tab.oid).unwrap();
        assert!(adapter.get_tab(&tab.oid).is_err());
    }

    #[test]
    fn test_layout_repository() {
        let adapter = make_adapter();

        let layout = LayoutState {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            focusednodeid: "n1".into(),
            rootnode: Some(serde_json::json!({"type": "leaf"})),
            ..Default::default()
        };

        adapter.save_layout(&layout).unwrap();
        let loaded = adapter.get_layout(&layout.oid).unwrap();
        assert_eq!(loaded.focusednodeid, "n1");
        assert!(loaded.rootnode.is_some());

        adapter.delete_layout(&layout.oid).unwrap();
        assert!(adapter.get_layout(&layout.oid).is_err());
    }

    #[test]
    fn test_object_store_unknown_otype_get() {
        let adapter = make_adapter();
        let result = adapter.get_object_json("foobar", "some-id");
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_object_store_unknown_otype_set() {
        let adapter = make_adapter();
        let result = adapter.set_object_json("foobar", "some-id", &serde_json::json!({}));
        assert!(matches!(result, Err(RepositoryError::NotFound(_))));
    }

    #[test]
    fn test_set_object_json_fills_empty_oid() {
        let adapter = make_adapter();

        let oid = uuid::Uuid::new_v4().to_string();
        let tab_json = serde_json::json!({
            "oid": "",
            "version": 1,
            "name": "Filled OID",
            "layoutstate": "",
            "blockids": [],
            "meta": {}
        });

        adapter.set_object_json(OTYPE_TAB, &oid, &tab_json).unwrap();
        let loaded = adapter.get_object_json(OTYPE_TAB, &oid).unwrap();
        assert_eq!(loaded["name"], "Filled OID");
    }

    #[test]
    fn test_upsert_update_existing() {
        let adapter = make_adapter();

        let mut ws = Workspace {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            name: "Original".into(),
            ..Default::default()
        };

        adapter.save_workspace(&ws).unwrap();

        ws.name = "Updated".into();
        ws.version = 2;
        adapter.save_workspace(&ws).unwrap();

        let loaded = adapter.get_workspace(&ws.oid).unwrap();
        assert_eq!(loaded.name, "Updated");
    }

    #[test]
    fn test_client_save_and_update() {
        let adapter = make_adapter();

        let mut client = Client {
            oid: uuid::Uuid::new_v4().to_string(),
            version: 1,
            windowids: vec!["w1".into()],
            meta: MetaMapType::new(),
            ..Default::default()
        };

        adapter.save_client(&client).unwrap();
        let loaded = adapter.get_client().unwrap();
        assert_eq!(loaded.windowids, vec!["w1"]);

        client.windowids.push("w2".into());
        client.version = 2;
        adapter.save_client(&client).unwrap();

        let loaded = adapter.get_client().unwrap();
        assert_eq!(loaded.windowids, vec!["w1", "w2"]);
    }

    #[test]
    fn test_list_workspaces_empty() {
        let adapter = make_adapter();
        let all = adapter.list_workspaces().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn test_list_workspaces_multiple() {
        let adapter = make_adapter();

        for i in 0..3 {
            let ws = Workspace {
                oid: uuid::Uuid::new_v4().to_string(),
                version: 1,
                name: format!("WS {i}"),
                ..Default::default()
            };
            adapter.save_workspace(&ws).unwrap();
        }

        let all = adapter.list_workspaces().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_store_err_conversion_variants() {
        // Test all StoreError → RepositoryError conversion paths
        let err = store_err_to_repo(StoreError::NotFound);
        assert!(matches!(err, RepositoryError::NotFound(_)));

        let err = store_err_to_repo(StoreError::AlreadyExists);
        assert!(matches!(err, RepositoryError::Constraint(_)));

        let err = store_err_to_repo(StoreError::EmptyOID);
        assert!(matches!(err, RepositoryError::Constraint(_)));

        let err = store_err_to_repo(StoreError::VersionMismatch {
            expected: 1,
            actual: 2,
        });
        assert!(matches!(err, RepositoryError::Constraint(_)));
        assert!(err.to_string().contains("version mismatch"));

        let err = store_err_to_repo(StoreError::Other("custom".into()));
        assert!(matches!(err, RepositoryError::Storage(_)));
    }

    #[test]
    fn test_inner_access() {
        let adapter = make_adapter();
        let _inner = adapter.inner();
        // Just verify inner() doesn't panic and returns a reference
    }
}
