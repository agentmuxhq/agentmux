# AgentMux Modular Architecture Refactoring Plan

**Status**: Planning
**Start Date**: February 10, 2026
**Target**: Version 0.21.0
**Related**: [ARCHITECTURE_ANALYSIS_2026-02-10.md](../ARCHITECTURE_ANALYSIS_2026-02-10.md)

---

## Overview

This document outlines the step-by-step plan to refactor AgentMux from a monolithic structure to a modular, layered architecture. The refactoring aims to:

1. **Improve testability** - Isolate business logic from infrastructure
2. **Enable code reuse** - Share logic between Tauri IPC, CLI, and tests
3. **Simplify debugging** - Clear boundaries between layers
4. **Facilitate collaboration** - Multiple agents can work on separate modules
5. **Catch bugs early** - Contract tests at layer boundaries

---

## Target Architecture

```
┌─────────────────────────────────────────────────────────┐
│  src-tauri/src/                                         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  adapters/  (Presentation Layer)                   │ │
│  │  • commands/ - Tauri IPC handlers (thin)           │ │
│  │  • cli/ - CLI commands                             │ │
│  │  • http/ - Future: REST API                        │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  services/  (Application Layer)                    │ │
│  │  • workspace_service.rs                            │ │
│  │  • tab_service.rs                                  │ │
│  │  • block_service.rs                                │ │
│  │  • agent_service.rs                                │ │
│  │  • config_service.rs                               │ │
│  │  • connection_service.rs                           │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  domain/  (Domain Layer)                           │ │
│  │  • entities/                                       │ │
│  │    - client.rs, window.rs, workspace.rs, tab.rs    │ │
│  │    - block.rs, layout.rs                           │ │
│  │  • value_objects/                                  │ │
│  │    - ids.rs (BlockId, TabId, etc.)                 │ │
│  │    - meta.rs (MetaMap)                             │ │
│  │    - connection.rs (ConnStatus, ConnOpts)          │ │
│  │  • events/                                         │ │
│  │    - events.rs (domain events)                     │ │
│  │  • traits/                                         │ │
│  │    - repository.rs (storage contracts)             │ │
│  └────────────────────────────────────────────────────┘ │
│                          ↕                               │
│  ┌────────────────────────────────────────────────────┐ │
│  │  infrastructure/  (Infrastructure Layer)           │ │
│  │  • storage/                                        │ │
│  │    - wavestore.rs, filestore.rs (SQLite)           │ │
│  │  • ipc/                                            │ │
│  │    - wsh_server.rs (named pipe IPC)                │ │
│  │  • pty/                                            │ │
│  │    - blockcontroller.rs (terminal management)      │ │
│  │  • config/                                         │ │
│  │    - loader.rs, watcher.rs                         │ │
│  │  • rpc/                                            │ │
│  │    - engine.rs, router.rs                          │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

---

## Dependency Rules

```
┌─────────────┐
│  adapters/  │  ─┐
└─────────────┘   │
                  ↓
┌─────────────┐   │
│  services/  │  ─┤   All layers can depend on domain
└─────────────┘   │
                  ↓
┌─────────────┐   │
│   domain/   │  ←┘
└─────────────┘
                  ↑
┌──────────────┐  │
│infrastructure│ ─┘   Infrastructure implements domain traits
└──────────────┘
```

**Rules**:
1. Domain has **zero dependencies** on other layers
2. Services depend **only on domain**
3. Adapters depend on services + domain
4. Infrastructure implements domain traits

---

## Phase 1: Extract Domain Layer

**Goal**: Create pure domain types with no Tauri/storage dependencies

**Branch**: `agenta/phase-1-domain-models`
**PR Number**: TBD
**Estimated Effort**: 2-3 hours

### Tasks

#### 1.1 Create Directory Structure

```bash
src-tauri/src/
  domain/
    mod.rs
    entities/
      mod.rs
      client.rs
      window.rs
      workspace.rs
      tab.rs
      block.rs
      layout.rs
    value_objects/
      mod.rs
      ids.rs
      meta.rs
      position.rs
    events/
      mod.rs
      events.rs
    traits/
      mod.rs
      repository.rs
```

#### 1.2 Extract Core Entities

**From**: `backend/waveobj.rs`
**To**: `domain/entities/*.rs`

**Types to Extract**:
- `Client` → `domain/entities/client.rs`
- `Window` → `domain/entities/window.rs`
- `Workspace` → `domain/entities/workspace.rs`
- `Tab` → `domain/entities/tab.rs`
- `Block` → `domain/entities/block.rs`
- `LayoutState` → `domain/entities/layout.rs`

**Pattern**:
```rust
// domain/entities/client.rs
use serde::{Deserialize, Serialize};
use crate::domain::value_objects::{ClientId, MetaMap};

/// Core Client entity representing a Wave client instance.
/// Pure domain model with no storage or presentation concerns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: ClientId,
    pub version: i64,
    pub window_ids: Vec<WindowId>,
    pub meta: MetaMap,
    pub tos_agreed: Option<i64>,
    pub has_old_history: bool,
    pub temp_oid: Option<String>,
}

impl Client {
    /// Create a new Client with defaults
    pub fn new(id: ClientId) -> Self {
        Self {
            id,
            version: 1,
            window_ids: Vec::new(),
            meta: MetaMap::new(),
            tos_agreed: None,
            has_old_history: false,
            temp_oid: None,
        }
    }

    /// Add a window to this client
    pub fn add_window(&mut self, window_id: WindowId) {
        if !self.window_ids.contains(&window_id) {
            self.window_ids.push(window_id);
            self.version += 1;
        }
    }

    /// Remove a window from this client
    pub fn remove_window(&mut self, window_id: &WindowId) -> bool {
        if let Some(pos) = self.window_ids.iter().position(|id| id == window_id) {
            self.window_ids.remove(pos);
            self.version += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let id = ClientId::new();
        let client = Client::new(id.clone());
        assert_eq!(client.id, id);
        assert_eq!(client.version, 1);
        assert!(client.window_ids.is_empty());
    }

    #[test]
    fn test_add_window() {
        let mut client = Client::new(ClientId::new());
        let window_id = WindowId::new();

        client.add_window(window_id.clone());
        assert_eq!(client.window_ids.len(), 1);
        assert_eq!(client.version, 2);

        // Adding same window again doesn't duplicate
        client.add_window(window_id.clone());
        assert_eq!(client.window_ids.len(), 1);
    }
}
```

#### 1.3 Extract Value Objects

**From**: `backend/waveobj.rs`
**To**: `domain/value_objects/*.rs`

**Types**:
```rust
// domain/value_objects/ids.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            pub fn from_string(s: String) -> Self {
                Self(s)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

define_id!(ClientId);
define_id!(WindowId);
define_id!(WorkspaceId);
define_id!(TabId);
define_id!(BlockId);
define_id!(LayoutNodeId);
```

#### 1.4 Define Repository Traits

```rust
// domain/traits/repository.rs
use crate::domain::entities::*;
use crate::domain::value_objects::*;

pub type Result<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub trait ClientRepository: Send + Sync {
    fn get(&self, id: &ClientId) -> Result<Client>;
    fn save(&self, client: &Client) -> Result<()>;
    fn delete(&self, id: &ClientId) -> Result<()>;
}

pub trait WindowRepository: Send + Sync {
    fn get(&self, id: &WindowId) -> Result<Window>;
    fn save(&self, window: &Window) -> Result<()>;
    fn delete(&self, id: &WindowId) -> Result<()>;
    fn list_by_client(&self, client_id: &ClientId) -> Result<Vec<Window>>;
}

pub trait WorkspaceRepository: Send + Sync {
    fn get(&self, id: &WorkspaceId) -> Result<Workspace>;
    fn save(&self, workspace: &Workspace) -> Result<()>;
    fn delete(&self, id: &WorkspaceId) -> Result<()>;
    fn list_all(&self) -> Result<Vec<Workspace>>;
}

pub trait TabRepository: Send + Sync {
    fn get(&self, id: &TabId) -> Result<Tab>;
    fn save(&self, tab: &Tab) -> Result<()>;
    fn delete(&self, id: &TabId) -> Result<()>;
    fn list_by_workspace(&self, workspace_id: &WorkspaceId) -> Result<Vec<Tab>>;
}

pub trait BlockRepository: Send + Sync {
    fn get(&self, id: &BlockId) -> Result<Block>;
    fn save(&self, block: &Block) -> Result<()>;
    fn delete(&self, id: &BlockId) -> Result<()>;
    fn list_by_tab(&self, tab_id: &TabId) -> Result<Vec<Block>>;
}
```

#### 1.5 Update lib.rs

```rust
// src-tauri/src/lib.rs
mod domain;
mod backend;  // Keep for now, will migrate in later phases
mod commands;
// ... rest
```

### Success Criteria

- [ ] All domain types compile without warnings
- [ ] Domain module has **zero** dependencies on:
  - `tauri`
  - `backend::storage`
  - `commands`
- [ ] Unit tests pass for all domain types
- [ ] Repository traits defined with clear contracts
- [ ] PR opened and passing CI

---

## Phase 2: Create Service Layer

**Goal**: Extract business logic into stateless services

**Branch**: `agenta/phase-2-services`
**PR Number**: TBD
**Estimated Effort**: 4-6 hours

### Tasks

#### 2.1 Create Service Structure

```
src-tauri/src/
  services/
    mod.rs
    workspace_service.rs
    tab_service.rs
    block_service.rs
    agent_service.rs
    config_service.rs
```

#### 2.2 Implement WorkspaceService

```rust
// services/workspace_service.rs
use crate::domain::{
    entities::{Workspace, Tab},
    value_objects::{WorkspaceId, TabId},
    traits::{WorkspaceRepository, TabRepository, RepositoryError},
};
use std::sync::Arc;

pub struct WorkspaceService {
    workspace_repo: Arc<dyn WorkspaceRepository>,
    tab_repo: Arc<dyn TabRepository>,
}

impl WorkspaceService {
    pub fn new(
        workspace_repo: Arc<dyn WorkspaceRepository>,
        tab_repo: Arc<dyn TabRepository>,
    ) -> Self {
        Self {
            workspace_repo,
            tab_repo,
        }
    }

    /// Get a workspace by ID with all its tabs
    pub fn get_workspace_with_tabs(
        &self,
        id: &WorkspaceId,
    ) -> Result<(Workspace, Vec<Tab>), RepositoryError> {
        let workspace = self.workspace_repo.get(id)?;
        let tabs = self.tab_repo.list_by_workspace(id)?;
        Ok((workspace, tabs))
    }

    /// Create a new workspace with a default tab
    pub fn create_workspace(&self, name: String, icon: String, color: String) -> Result<Workspace, RepositoryError> {
        let workspace_id = WorkspaceId::new();
        let tab_id = TabId::new();

        // Create default tab
        let tab = Tab::new(tab_id.clone(), "New Tab".to_string());
        self.tab_repo.save(&tab)?;

        // Create workspace
        let mut workspace = Workspace::new(workspace_id, name);
        workspace.icon = icon;
        workspace.color = color;
        workspace.tab_ids = vec![tab_id.clone()];
        workspace.active_tab_id = Some(tab_id);

        self.workspace_repo.save(&workspace)?;

        Ok(workspace)
    }

    /// Delete a workspace and all its tabs
    pub fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), RepositoryError> {
        // Get tabs first
        let tabs = self.tab_repo.list_by_workspace(id)?;

        // Delete all tabs
        for tab in tabs {
            self.tab_repo.delete(&tab.id)?;
        }

        // Delete workspace
        self.workspace_repo.delete(id)?;

        Ok(())
    }

    /// Set the active tab for a workspace
    pub fn set_active_tab(
        &self,
        workspace_id: &WorkspaceId,
        tab_id: &TabId,
    ) -> Result<(), RepositoryError> {
        let mut workspace = self.workspace_repo.get(workspace_id)?;

        // Verify tab belongs to workspace
        if !workspace.tab_ids.contains(tab_id) {
            return Err(RepositoryError::NotFound(format!(
                "Tab {} not in workspace {}",
                tab_id, workspace_id
            )));
        }

        workspace.active_tab_id = Some(tab_id.clone());
        workspace.version += 1;

        self.workspace_repo.save(&workspace)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Mock repository for testing
    struct MockWorkspaceRepository {
        workspaces: Mutex<HashMap<String, Workspace>>,
    }

    impl MockWorkspaceRepository {
        fn new() -> Self {
            Self {
                workspaces: Mutex::new(HashMap::new()),
            }
        }
    }

    impl WorkspaceRepository for MockWorkspaceRepository {
        fn get(&self, id: &WorkspaceId) -> Result<Workspace, RepositoryError> {
            self.workspaces
                .lock()
                .unwrap()
                .get(id.as_str())
                .cloned()
                .ok_or_else(|| RepositoryError::NotFound(id.to_string()))
        }

        fn save(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
            self.workspaces
                .lock()
                .unwrap()
                .insert(workspace.id.as_str().to_string(), workspace.clone());
            Ok(())
        }

        fn delete(&self, id: &WorkspaceId) -> Result<(), RepositoryError> {
            self.workspaces
                .lock()
                .unwrap()
                .remove(id.as_str())
                .ok_or_else(|| RepositoryError::NotFound(id.to_string()))?;
            Ok(())
        }

        fn list_all(&self) -> Result<Vec<Workspace>, RepositoryError> {
            Ok(self.workspaces.lock().unwrap().values().cloned().collect())
        }
    }

    // Similar mock for TabRepository...

    #[test]
    fn test_create_workspace() {
        let workspace_repo = Arc::new(MockWorkspaceRepository::new());
        let tab_repo = Arc::new(MockTabRepository::new());
        let service = WorkspaceService::new(workspace_repo.clone(), tab_repo);

        let workspace = service
            .create_workspace(
                "Test Workspace".to_string(),
                "test-icon".to_string(),
                "#FF0000".to_string(),
            )
            .unwrap();

        assert_eq!(workspace.name, "Test Workspace");
        assert_eq!(workspace.tab_ids.len(), 1);
        assert!(workspace.active_tab_id.is_some());

        // Verify saved
        let loaded = workspace_repo.get(&workspace.id).unwrap();
        assert_eq!(loaded.id, workspace.id);
    }

    #[test]
    fn test_delete_workspace() {
        let workspace_repo = Arc::new(MockWorkspaceRepository::new());
        let tab_repo = Arc::new(MockTabRepository::new());
        let service = WorkspaceService::new(workspace_repo.clone(), tab_repo.clone());

        let workspace = service.create_workspace(
            "Test".to_string(),
            "icon".to_string(),
            "#FF0000".to_string(),
        ).unwrap();

        // Delete
        service.delete_workspace(&workspace.id).unwrap();

        // Verify deleted
        assert!(workspace_repo.get(&workspace.id).is_err());
    }
}
```

### Success Criteria

- [ ] All services compile without warnings
- [ ] Services depend **only** on domain layer
- [ ] All services have 80%+ unit test coverage with mocks
- [ ] Business logic is isolated from storage/presentation
- [ ] PR opened and passing CI

---

## Phase 3: Refactor IPC Commands

**Goal**: Make command handlers thin adapters that delegate to services

**Branch**: `agenta/phase-3-thin-commands`
**PR Number**: TBD
**Estimated Effort**: 3-4 hours

### Tasks

#### 3.1 Create Adapter Layer

```
src-tauri/src/
  adapters/
    commands/
      mod.rs
      workspace.rs
      tab.rs
      block.rs
      agent.rs
```

#### 3.2 Refactor Workspace Commands

**Before** (`commands/rpc.rs`):
```rust
#[tauri::command]
pub fn service_request(data: Value, state: State<AppState>) -> Result<Value, String> {
    // 200 lines of mixed logic and data access
    let workspace_id = data["workspaceId"].as_str().unwrap();
    let store = &state.wave_store;
    let workspace = store.get_workspace(workspace_id)?;
    // ... more logic
}
```

**After** (`adapters/commands/workspace.rs`):
```rust
use crate::domain::value_objects::WorkspaceId;
use crate::services::WorkspaceService;
use crate::adapters::dto::WorkspaceDto;
use tauri::State;
use std::sync::Arc;

#[tauri::command]
pub fn get_workspace(
    workspace_id: String,
    service: State<Arc<WorkspaceService>>,
) -> Result<WorkspaceDto, String> {
    let id = WorkspaceId::from(workspace_id);
    let (workspace, tabs) = service
        .get_workspace_with_tabs(&id)
        .map_err(|e| e.to_string())?;

    Ok(WorkspaceDto::from_domain(workspace, tabs))
}

#[tauri::command]
pub fn create_workspace(
    name: String,
    icon: String,
    color: String,
    service: State<Arc<WorkspaceService>>,
) -> Result<WorkspaceDto, String> {
    let workspace = service
        .create_workspace(name, icon, color)
        .map_err(|e| e.to_string())?;

    Ok(WorkspaceDto::from_domain(workspace, vec![]))
}

#[tauri::command]
pub fn delete_workspace(
    workspace_id: String,
    service: State<Arc<WorkspaceService>>,
) -> Result<(), String> {
    let id = WorkspaceId::from(workspace_id);
    service.delete_workspace(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_tab(
    workspace_id: String,
    tab_id: String,
    service: State<Arc<WorkspaceService>>,
) -> Result<(), String> {
    let wid = WorkspaceId::from(workspace_id);
    let tid = TabId::from(tab_id);
    service.set_active_tab(&wid, &tid).map_err(|e| e.to_string())
}
```

#### 3.3 Create DTOs

```rust
// adapters/dto/workspace.rs
use crate::domain::entities::{Workspace, Tab};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceDto {
    pub oid: String,
    pub otype: String,
    pub version: i64,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub tabids: Vec<String>,
    pub activetabid: Option<String>,
    pub pinnedtabids: Vec<String>,
    pub meta: serde_json::Value,
}

impl WorkspaceDto {
    pub fn from_domain(workspace: Workspace, _tabs: Vec<Tab>) -> Self {
        Self {
            oid: workspace.id.into_string(),
            otype: "workspace".to_string(),
            version: workspace.version,
            name: workspace.name,
            icon: workspace.icon,
            color: workspace.color,
            tabids: workspace.tab_ids.into_iter().map(|id| id.into_string()).collect(),
            activetabid: workspace.active_tab_id.map(|id| id.into_string()),
            pinnedtabids: workspace.pinned_tab_ids.into_iter().map(|id| id.into_string()).collect(),
            meta: serde_json::to_value(&workspace.meta).unwrap_or_default(),
        }
    }
}
```

#### 3.4 Update lib.rs Setup

```rust
// lib.rs
use services::*;
use infrastructure::storage::{WaveStoreImpl, FileStoreImpl};

.setup(|app| {
    // Create infrastructure
    let wave_store = Arc::new(WaveStoreImpl::new(&data_dir)?);
    let file_store = Arc::new(FileStoreImpl::new(&data_dir)?);

    // Create services
    let workspace_service = Arc::new(WorkspaceService::new(
        Arc::clone(&wave_store) as Arc<dyn WorkspaceRepository>,
        Arc::clone(&wave_store) as Arc<dyn TabRepository>,
    ));

    // Manage services in Tauri state
    app.manage(workspace_service);
    app.manage(tab_service);
    app.manage(block_service);
    // ...
})
.invoke_handler(tauri::generate_handler![
    adapters::commands::workspace::get_workspace,
    adapters::commands::workspace::create_workspace,
    adapters::commands::workspace::delete_workspace,
    adapters::commands::workspace::set_active_tab,
    // ...
])
```

### Success Criteria

- [ ] All command handlers are <50 lines
- [ ] Commands only do: parse input → call service → map output
- [ ] No business logic in command layer
- [ ] DTOs clearly separate domain from presentation
- [ ] PR opened and passing CI

---

## Phase 4: Implement Infrastructure Layer

**Goal**: Make storage implement domain traits

**Branch**: `agenta/phase-4-infrastructure`
**PR Number**: TBD
**Estimated Effort**: 5-7 hours

### Tasks

#### 4.1 Move Infrastructure

```
src-tauri/src/
  infrastructure/
    storage/
      mod.rs
      wavestore_impl.rs    (implements all repository traits)
      filestore_impl.rs
      migrations.rs
    ipc/
      wsh_server.rs
    pty/
      blockcontroller.rs
    config/
      loader.rs
      watcher.rs
    rpc/
      engine.rs
      router.rs
```

#### 4.2 Implement Repository Traits

```rust
// infrastructure/storage/wavestore_impl.rs
use crate::domain::{
    entities::Workspace,
    value_objects::WorkspaceId,
    traits::{WorkspaceRepository, RepositoryError},
};
use rusqlite::{Connection, params};
use std::sync::Mutex;

pub struct WaveStoreImpl {
    conn: Mutex<Connection>,
}

impl WaveStoreImpl {
    pub fn new(path: &Path) -> Result<Self, RepositoryError> {
        let conn = Connection::open(path)
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl WorkspaceRepository for WaveStoreImpl {
    fn get(&self, id: &WorkspaceId) -> Result<Workspace, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM workspace WHERE oid = ?1")
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let json: String = stmt
            .query_row(params![id.as_str()], |row| row.get(0))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    RepositoryError::NotFound(id.to_string())
                }
                _ => RepositoryError::Storage(e.to_string()),
            })?;

        serde_json::from_str(&json)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))
    }

    fn save(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(workspace)
            .map_err(|e| RepositoryError::Serialization(e.to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO workspace (oid, data) VALUES (?1, ?2)",
            params![workspace.id.as_str(), json],
        )
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        Ok(())
    }

    fn delete(&self, id: &WorkspaceId) -> Result<(), RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let rows = conn
            .execute("DELETE FROM workspace WHERE oid = ?1", params![id.as_str()])
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        if rows == 0 {
            Err(RepositoryError::NotFound(id.to_string()))
        } else {
            Ok(())
        }
    }

    fn list_all(&self) -> Result<Vec<Workspace>, RepositoryError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT data FROM workspace")
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;

        let mut workspaces = Vec::new();
        for row in rows {
            let json = row.map_err(|e| RepositoryError::Storage(e.to_string()))?;
            let workspace: Workspace = serde_json::from_str(&json)
                .map_err(|e| RepositoryError::Serialization(e.to_string()))?;
            workspaces.push(workspace);
        }

        Ok(workspaces)
    }
}
```

### Success Criteria

- [ ] All storage implementations moved to infrastructure/
- [ ] All repository traits implemented
- [ ] Integration tests pass with real SQLite
- [ ] No circular dependencies between layers
- [ ] PR opened and passing CI

---

## Phase 5: Frontend Modularization

**Goal**: Separate frontend state management from RPC layer

**Branch**: `agenta/phase-5-frontend-services`
**PR Number**: TBD
**Estimated Effort**: 4-6 hours

### Tasks

#### 5.1 Create Frontend Service Layer

```
frontend/app/
  services/
    workspace_service.ts
    tab_service.ts
    block_service.ts
    rpc_client.ts
  models/
    workspace.ts
    tab.ts
    block.ts
  hooks/
    useWorkspace.ts
    useTab.ts
    useBlock.ts
```

#### 5.2 Implement Typed RPC Client

```typescript
// services/rpc_client.ts
import { invoke } from '@tauri-apps/api/core';

export class RpcClient {
    async call<T>(command: string, params?: any): Promise<T> {
        try {
            return await invoke(command, params);
        } catch (error) {
            console.error(`RPC call failed: ${command}`, error);
            throw error;
        }
    }
}

export const rpcClient = new RpcClient();
```

#### 5.3 Implement Frontend Services

```typescript
// services/workspace_service.ts
import { rpcClient } from './rpc_client';
import { Workspace } from '../models/workspace';

export class WorkspaceService {
    async getWorkspace(id: string): Promise<Workspace> {
        const dto = await rpcClient.call('get_workspace', { workspaceId: id });
        return Workspace.fromDto(dto);
    }

    async createWorkspace(name: string, icon: string, color: string): Promise<Workspace> {
        const dto = await rpcClient.call('create_workspace', { name, icon, color });
        return Workspace.fromDto(dto);
    }

    async deleteWorkspace(id: string): Promise<void> {
        await rpcClient.call('delete_workspace', { workspaceId: id });
    }

    async setActiveTab(workspaceId: string, tabId: string): Promise<void> {
        await rpcClient.call('set_active_tab', { workspaceId, tabId });
    }
}

export const workspaceService = new WorkspaceService();
```

#### 5.4 Create React Hooks

```typescript
// hooks/useWorkspace.ts
import { useState, useEffect } from 'react';
import { workspaceService } from '../services/workspace_service';
import { Workspace } from '../models/workspace';

export function useWorkspace(workspaceId: string) {
    const [workspace, setWorkspace] = useState<Workspace | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<Error | null>(null);

    useEffect(() => {
        let cancelled = false;

        async function load() {
            try {
                const ws = await workspaceService.getWorkspace(workspaceId);
                if (!cancelled) {
                    setWorkspace(ws);
                    setLoading(false);
                }
            } catch (err) {
                if (!cancelled) {
                    setError(err as Error);
                    setLoading(false);
                }
            }
        }

        load();

        return () => {
            cancelled = true;
        };
    }, [workspaceId]);

    return { workspace, loading, error };
}
```

### Success Criteria

- [ ] All frontend services implemented
- [ ] React hooks use services, not direct RPC
- [ ] Type safety end-to-end (Rust → TypeScript)
- [ ] Components simplified (no RPC calls in components)
- [ ] PR opened and passing CI

---

## Phase 6: Testing Infrastructure

**Goal**: Add comprehensive test coverage

**Branch**: `agenta/phase-6-testing`
**PR Number**: TBD
**Estimated Effort**: 3-4 hours

### Tasks

#### 6.1 Domain Layer Tests

```rust
// domain/entities/workspace.rs
#[cfg(test)]
mod tests {
    #[test]
    fn test_workspace_creation() {
        let id = WorkspaceId::new();
        let ws = Workspace::new(id.clone(), "Test".to_string());
        assert_eq!(ws.id, id);
        assert_eq!(ws.name, "Test");
        assert_eq!(ws.version, 1);
    }

    #[test]
    fn test_add_tab() {
        let mut ws = Workspace::new(WorkspaceId::new(), "Test".to_string());
        let tab_id = TabId::new();

        ws.add_tab(tab_id.clone());
        assert_eq!(ws.tab_ids.len(), 1);
        assert_eq!(ws.version, 2);
    }
}
```

#### 6.2 Service Layer Tests with Mocks

```rust
// services/workspace_service.rs
#[cfg(test)]
mod tests {
    use mockall::predicate::*;
    use mockall::mock;

    mock! {
        WorkspaceRepo {}
        impl WorkspaceRepository for WorkspaceRepo {
            fn get(&self, id: &WorkspaceId) -> Result<Workspace, RepositoryError>;
            fn save(&self, workspace: &Workspace) -> Result<(), RepositoryError>;
            fn delete(&self, id: &WorkspaceId) -> Result<(), RepositoryError>;
            fn list_all(&self) -> Result<Vec<Workspace>, RepositoryError>;
        }
    }

    #[test]
    fn test_create_workspace() {
        let mut mock_workspace_repo = MockWorkspaceRepo::new();
        let mut mock_tab_repo = MockTabRepo::new();

        // Set up expectations
        mock_tab_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        mock_workspace_repo
            .expect_save()
            .times(1)
            .returning(|_| Ok(()));

        let service = WorkspaceService::new(
            Arc::new(mock_workspace_repo),
            Arc::new(mock_tab_repo),
        );

        let result = service.create_workspace(
            "Test".to_string(),
            "icon".to_string(),
            "#FF0000".to_string(),
        );

        assert!(result.is_ok());
    }
}
```

#### 6.3 Integration Tests

```rust
// tests/integration_tests.rs
use agentmux::infrastructure::storage::WaveStoreImpl;
use agentmux::services::WorkspaceService;
use tempfile::TempDir;

#[test]
fn test_workspace_service_with_real_db() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let store = Arc::new(WaveStoreImpl::new(&db_path).unwrap());
    let service = WorkspaceService::new(
        Arc::clone(&store) as Arc<dyn WorkspaceRepository>,
        Arc::clone(&store) as Arc<dyn TabRepository>,
    );

    // Create workspace
    let workspace = service
        .create_workspace("Test".to_string(), "icon".to_string(), "#FF0000".to_string())
        .unwrap();

    // Verify it can be retrieved
    let (loaded, tabs) = service
        .get_workspace_with_tabs(&workspace.id)
        .unwrap();

    assert_eq!(loaded.id, workspace.id);
    assert_eq!(loaded.name, "Test");
    assert_eq!(tabs.len(), 1);
}
```

### Success Criteria

- [ ] Domain layer: 90%+ coverage
- [ ] Service layer: 80%+ coverage
- [ ] Integration tests cover happy paths
- [ ] CI runs all tests on each PR
- [ ] Test runtime <30 seconds

---

## Phase 7: Documentation & Cleanup

**Goal**: Document the new architecture and remove old code

**Branch**: `agenta/phase-7-docs-cleanup`
**PR Number**: TBD
**Estimated Effort**: 2-3 hours

### Tasks

#### 7.1 Update Documentation

- [ ] Update BUILD.md with new architecture
- [ ] Create ARCHITECTURE.md with layer diagrams
- [ ] Add README to each module explaining its purpose
- [ ] Document repository trait contracts
- [ ] Add examples to service docstrings

#### 7.2 Remove Old Code

- [ ] Delete old `backend/waveobj.rs` (replaced by domain/)
- [ ] Delete old RPC handlers (replaced by services)
- [ ] Remove unused dependencies from Cargo.toml
- [ ] Clean up commented-out code

#### 7.3 Performance Audit

- [ ] Profile app startup time
- [ ] Check memory usage
- [ ] Ensure no regressions vs 0.20.x

### Success Criteria

- [ ] All documentation updated
- [ ] No dead code in repo
- [ ] Performance equal or better than 0.20.x
- [ ] PR merged and released as 0.21.0

---

## Migration Strategy

### Backwards Compatibility

During the refactoring, we'll maintain backwards compatibility by:

1. **Dual Code Paths**: Keep old handlers alongside new ones
2. **Feature Flags**: Use Cargo features to toggle between old/new
3. **Gradual Migration**: Migrate one command at a time
4. **Database Schema**: No schema changes during refactoring

### Testing Strategy

1. **Unit Tests**: Test each layer in isolation
2. **Integration Tests**: Test full stack with real DB
3. **E2E Tests**: Playwright tests remain unchanged
4. **Manual Testing**: Test each PR build before merge

### Rollback Plan

If a phase causes issues:

1. **Revert PR**: Git revert the merge commit
2. **Fix Forward**: Create hotfix PR on top
3. **Skip Phase**: Mark phase as blocked, continue to next

---

## Success Metrics

### Code Quality

- **Test Coverage**: >80% overall
- **Lines of Code**: Decrease by 10-15% (less duplication)
- **Cyclomatic Complexity**: Decrease by 20% (simpler functions)
- **Build Time**: No increase (parallel compilation)

### Developer Experience

- **Onboarding**: New devs can understand architecture in <1 hour
- **Debugging**: Can debug issues without reading full codebase
- **Testing**: Can write unit tests for new features in <10 minutes
- **Collaboration**: Multiple agents can work on different modules without conflicts

### Performance

- **Startup Time**: No regression
- **Memory Usage**: No increase
- **RPC Latency**: No increase
- **Bundle Size**: No significant increase

---

## Timeline

**Total Estimated Time**: 23-33 hours

| Phase | Effort | Dependencies |
|-------|--------|--------------|
| 1. Domain Layer | 2-3h | None |
| 2. Service Layer | 4-6h | Phase 1 |
| 3. IPC Commands | 3-4h | Phase 2 |
| 4. Infrastructure | 5-7h | Phase 1, 2 |
| 5. Frontend | 4-6h | Phase 3 |
| 6. Testing | 3-4h | All previous |
| 7. Docs & Cleanup | 2-3h | All previous |

**Phases 1 and 4 can run in parallel** (different team members/agents).
**Phases 2 and 5 can run in parallel** (backend vs frontend).

**Target Completion**: ~2-3 days with multiple agents working concurrently.

---

## Next Steps

1. **Review this spec** - Get feedback from all agents
2. **Start Phase 1** - Create `agenta/phase-1-domain-models` branch
3. **Open tracking issue** - GitHub issue to track all PRs
4. **Assign phases** - Distribute work across agents

---

## Notes

- This plan is **living documentation** - update as we learn
- Each phase should be **independently reviewable** - small PRs
- Focus on **incremental value** - each phase improves the codebase
- **Don't block on perfection** - iterate and improve over time

---

*Last Updated*: February 10, 2026
*Owner*: AgentA
*Status*: ✅ Ready to Start
