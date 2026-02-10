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
    fn test_event_object_updated_serde() {
        let event = DomainEvent::ObjectUpdated {
            otype: "workspace".to_string(),
            oid: "ws-123".to_string(),
            version: 5,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ObjectUpdated"));
        assert!(json.contains("ws-123"));

        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            DomainEvent::ObjectUpdated { otype, oid, version } => {
                assert_eq!(otype, "workspace");
                assert_eq!(oid, "ws-123");
                assert_eq!(version, 5);
            }
            _ => panic!("expected ObjectUpdated"),
        }
    }

    #[test]
    fn test_event_object_deleted_serde() {
        let event = DomainEvent::ObjectDeleted {
            otype: "block".to_string(),
            oid: "b-456".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            DomainEvent::ObjectDeleted { otype, oid } => {
                assert_eq!(otype, "block");
                assert_eq!(oid, "b-456");
            }
            _ => panic!("expected ObjectDeleted"),
        }
    }

    #[test]
    fn test_event_tab_activated_serde() {
        let event = DomainEvent::TabActivated {
            workspace_id: "ws-1".to_string(),
            tab_id: "t-1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            DomainEvent::TabActivated { workspace_id, tab_id } => {
                assert_eq!(workspace_id, "ws-1");
                assert_eq!(tab_id, "t-1");
            }
            _ => panic!("expected TabActivated"),
        }
    }

    #[test]
    fn test_event_block_added_serde() {
        let event = DomainEvent::BlockAdded {
            tab_id: "t-1".to_string(),
            block_id: "b-1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            DomainEvent::BlockAdded { tab_id, block_id } => {
                assert_eq!(tab_id, "t-1");
                assert_eq!(block_id, "b-1");
            }
            _ => panic!("expected BlockAdded"),
        }
    }

    #[test]
    fn test_event_block_removed_serde() {
        let event = DomainEvent::BlockRemoved {
            tab_id: "t-2".to_string(),
            block_id: "b-2".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        match parsed {
            DomainEvent::BlockRemoved { tab_id, block_id } => {
                assert_eq!(tab_id, "t-2");
                assert_eq!(block_id, "b-2");
            }
            _ => panic!("expected BlockRemoved"),
        }
    }

    #[test]
    fn test_event_clone_and_debug() {
        let event = DomainEvent::ObjectUpdated {
            otype: "tab".into(),
            oid: "t-1".into(),
            version: 1,
        };
        let cloned = event.clone();
        let debug = format!("{:?}", cloned);
        assert!(debug.contains("ObjectUpdated"));
    }
}
