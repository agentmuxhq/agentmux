// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Domain events — notifications about entity state changes.
//! These are emitted by services and consumed by adapters (IPC, pub/sub).

use serde::{Deserialize, Serialize};

/// A domain event representing a change to a WaveObj.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    /// An object was created or updated.
    ObjectUpdated {
        otype: String,
        oid: String,
        version: i64,
    },
    /// An object was deleted.
    ObjectDeleted { otype: String, oid: String },
    /// A tab was activated in a workspace.
    TabActivated {
        workspace_id: String,
        tab_id: String,
    },
    /// A block was added to a tab.
    BlockAdded { tab_id: String, block_id: String },
    /// A block was removed from a tab.
    BlockRemoved { tab_id: String, block_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = DomainEvent::ObjectUpdated {
            otype: "workspace".to_string(),
            oid: "ws-123".to_string(),
            version: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ObjectUpdated"));
        assert!(json.contains("ws-123"));
    }
}
