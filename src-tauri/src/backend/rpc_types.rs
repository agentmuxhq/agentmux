// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! RPC wire format types: Rust equivalents of Go structs from
//! pkg/wshutil/wshrpc.go and pkg/wshrpc/wshrpctypes.go.
//! Only the core types needed for the data layer are ported here;
//! the full command set will be added incrementally.

use serde::{Deserialize, Serialize};

use super::oref::ORef;
use super::waveobj::MetaMapType;

// ---- RpcMessage wire format ----

/// Matches Go's `wshutil.RpcMessage` from pkg/wshutil/wshrpc.go.
/// This is the on-the-wire JSON envelope for all RPC communication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RpcMessage {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reqid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resid: String,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub timeout: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authtoken: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cont: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cancel: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub datatype: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcMessage {
    pub fn is_rpc_request(&self) -> bool {
        !self.command.is_empty() || !self.reqid.is_empty()
    }

    /// Validates the packet structure. Matches Go's `RpcMessage.Validate()`.
    pub fn validate(&self) -> Result<(), String> {
        if !self.reqid.is_empty() && !self.resid.is_empty() {
            return Err("request packets may not have both reqid and resid set".into());
        }
        if self.cancel {
            if !self.command.is_empty() {
                return Err("cancel packets may not have command set".into());
            }
            if self.reqid.is_empty() && self.resid.is_empty() {
                return Err("cancel packets must have reqid or resid set".into());
            }
            if self.data.is_some() {
                return Err("cancel packets may not have data set".into());
            }
            return Ok(());
        }
        if !self.command.is_empty() {
            if !self.resid.is_empty() {
                return Err("command packets may not have resid set".into());
            }
            if !self.error.is_empty() {
                return Err("command packets may not have error set".into());
            }
            if !self.datatype.is_empty() {
                return Err("command packets may not have datatype set".into());
            }
            return Ok(());
        }
        if !self.reqid.is_empty() {
            if self.resid.is_empty() {
                return Err("request packets must have resid set".into());
            }
            if self.timeout != 0 {
                return Err("non-command request packets may not have timeout set".into());
            }
            return Ok(());
        }
        if !self.resid.is_empty() {
            if !self.command.is_empty() {
                return Err("response packets may not have command set".into());
            }
            if self.reqid.is_empty() {
                return Err("response packets must have reqid set".into());
            }
            if self.timeout != 0 {
                return Err("response packets may not have timeout set".into());
            }
            return Ok(());
        }
        Err("invalid packet: must have command, reqid, or resid set".into())
    }
}

// ---- Command constants ----

pub const COMMAND_GET_META: &str = "getmeta";
pub const COMMAND_SET_META: &str = "setmeta";
pub const COMMAND_MESSAGE: &str = "message";

// ---- Command data types (subset) ----

/// Matches Go's `CommandGetMetaData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandGetMetaData {
    pub oref: ORef,
}

/// Matches Go's `CommandSetMetaData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSetMetaData {
    pub oref: ORef,
    pub meta: MetaMapType,
}

/// Matches Go's `CommandMessageData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMessageData {
    pub oref: ORef,
    pub message: String,
}

/// Matches Go's `RpcOpts`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RpcOpts {
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub timeout: i64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub noresponse: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub route: String,
}

/// Matches Go's `RpcContext`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RpcContext {
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "ctype")]
    pub client_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub blockid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tabid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub conn: String,
}

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_message_command_roundtrip() {
        let msg = RpcMessage {
            command: "getmeta".to_string(),
            reqid: "req-123".to_string(),
            timeout: 5000,
            data: Some(serde_json::json!({"oref": "block:abc-123"})),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: RpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.command, "getmeta");
        assert_eq!(parsed.reqid, "req-123");
        assert_eq!(parsed.timeout, 5000);
        assert!(parsed.data.is_some());
    }

    #[test]
    fn test_rpc_message_response_roundtrip() {
        let msg = RpcMessage {
            reqid: "req-123".to_string(),
            resid: "res-456".to_string(),
            data: Some(serde_json::json!({"view": "term"})),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: RpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reqid, "req-123");
        assert_eq!(parsed.resid, "res-456");
    }

    #[test]
    fn test_rpc_message_empty_fields_omitted() {
        let msg = RpcMessage {
            command: "test".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("reqid"));
        assert!(!json.contains("resid"));
        assert!(!json.contains("timeout"));
        assert!(!json.contains("cont"));
        assert!(!json.contains("cancel"));
    }

    #[test]
    fn test_rpc_message_validate_command() {
        let msg = RpcMessage {
            command: "getmeta".to_string(),
            ..Default::default()
        };
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_rpc_message_validate_cancel() {
        let msg = RpcMessage {
            cancel: true,
            reqid: "req-1".to_string(),
            ..Default::default()
        };
        assert!(msg.validate().is_ok());

        // cancel without reqid or resid
        let bad = RpcMessage {
            cancel: true,
            ..Default::default()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn test_rpc_message_validate_empty() {
        let msg = RpcMessage::default();
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_rpc_message_validate_both_ids() {
        let msg = RpcMessage {
            reqid: "a".to_string(),
            resid: "b".to_string(),
            ..Default::default()
        };
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_command_get_meta_data() {
        let data = CommandGetMetaData {
            oref: ORef::new("block", "550e8400-e29b-41d4-a716-446655440000"),
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: CommandGetMetaData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.oref.otype, "block");
    }

    #[test]
    fn test_command_set_meta_data() {
        let mut meta = MetaMapType::new();
        meta.insert("view".into(), serde_json::json!("term"));

        let data = CommandSetMetaData {
            oref: ORef::new("block", "550e8400-e29b-41d4-a716-446655440000"),
            meta,
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: CommandSetMetaData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meta["view"], "term");
    }

    #[test]
    fn test_wire_compat_go_rpc_message() {
        // Simulated Go-produced JSON
        let go_json = r#"{"command":"getmeta","reqid":"abc","timeout":5000,"data":{"oref":"block:123"}}"#;
        let msg: RpcMessage = serde_json::from_str(go_json).unwrap();
        assert_eq!(msg.command, "getmeta");
        assert_eq!(msg.reqid, "abc");
        assert_eq!(msg.timeout, 5000);
    }

    #[test]
    fn test_rpc_context_roundtrip() {
        let ctx = RpcContext {
            client_type: "connserver".to_string(),
            blockid: "blk-1".to_string(),
            tabid: "tab-1".to_string(),
            conn: "local".to_string(),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains(r#""ctype":"connserver""#));
        let parsed: RpcContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.client_type, "connserver");
    }
}
