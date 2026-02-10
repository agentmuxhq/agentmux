// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! MetaMap: flexible key-value metadata map matching Go's MetaMapType.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Matches Go's `MetaMapType = map[string]any`.
pub type MetaMapType = HashMap<String, serde_json::Value>;

/// Merge `update` into `base`, matching Go's `MergeMeta` logic.
/// - Keys ending in `:*` with truthy value clear the section.
/// - `null` values delete the key.
/// - If `merge_special` is false, keys starting with `display:` are skipped.
pub fn merge_meta(base: &MetaMapType, update: &MetaMapType, merge_special: bool) -> MetaMapType {
    let mut result = base.clone();

    // First pass: handle "section:*" clear keys
    for (k, v) in update {
        if !k.ends_with(":*") {
            continue;
        }
        let is_true = matches!(v, serde_json::Value::Bool(true));
        if !is_true {
            continue;
        }
        let prefix = k.trim_end_matches(":*");
        if prefix.is_empty() {
            continue;
        }
        let prefix_colon = format!("{prefix}:");
        result.retain(|k2, _| k2 != prefix && !k2.starts_with(&prefix_colon));
    }

    // Second pass: merge regular keys
    for (k, v) in update {
        if !merge_special && k.starts_with("display:") {
            continue;
        }
        if k.ends_with(":*") {
            continue;
        }
        if v.is_null() {
            result.remove(k);
            continue;
        }
        result.insert(k.clone(), v.clone());
    }

    result
}

/// Helper to get a string value from MetaMapType.
pub fn meta_get_string(meta: &MetaMapType, key: &str, default: &str) -> String {
    meta.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

/// Helper to get a bool value from MetaMapType.
pub fn meta_get_bool(meta: &MetaMapType, key: &str, default: bool) -> bool {
    meta.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

/// Update types for WaveObj change notifications.
pub const UPDATE_TYPE_UPDATE: &str = "update";
pub const UPDATE_TYPE_DELETE: &str = "delete";

/// Represents an update notification for a wave object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveObjUpdate {
    pub updatetype: String,
    pub otype: String,
    pub oid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obj: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_meta_basic() {
        let mut base = MetaMapType::new();
        base.insert("view".into(), serde_json::json!("term"));
        base.insert("cmd".into(), serde_json::json!("ls"));

        let mut update = MetaMapType::new();
        update.insert("cmd".into(), serde_json::json!("pwd"));
        update.insert("icon".into(), serde_json::json!("star"));

        let result = merge_meta(&base, &update, true);
        assert_eq!(result["view"], "term");
        assert_eq!(result["cmd"], "pwd");
        assert_eq!(result["icon"], "star");
    }

    #[test]
    fn test_merge_meta_null_deletes() {
        let mut base = MetaMapType::new();
        base.insert("view".into(), serde_json::json!("term"));
        base.insert("cmd".into(), serde_json::json!("ls"));

        let mut update = MetaMapType::new();
        update.insert("cmd".into(), serde_json::Value::Null);

        let result = merge_meta(&base, &update, true);
        assert_eq!(result.get("view").unwrap(), "term");
        assert!(!result.contains_key("cmd"));
    }

    #[test]
    fn test_merge_meta_section_clear() {
        let mut base = MetaMapType::new();
        base.insert("frame".into(), serde_json::json!(true));
        base.insert("frame:title".into(), serde_json::json!("hello"));
        base.insert("frame:icon".into(), serde_json::json!("star"));
        base.insert("cmd".into(), serde_json::json!("ls"));

        let mut update = MetaMapType::new();
        update.insert("frame:*".into(), serde_json::json!(true));

        let result = merge_meta(&base, &update, true);
        assert!(!result.contains_key("frame"));
        assert!(!result.contains_key("frame:title"));
        assert_eq!(result["cmd"], "ls");
    }

    #[test]
    fn test_merge_meta_skip_display() {
        let base = MetaMapType::new();
        let mut update = MetaMapType::new();
        update.insert("display:name".into(), serde_json::json!("test"));
        update.insert("view".into(), serde_json::json!("term"));

        let result = merge_meta(&base, &update, false);
        assert!(!result.contains_key("display:name"));
        assert_eq!(result["view"], "term");
    }

    #[test]
    fn test_meta_helpers() {
        let mut meta = MetaMapType::new();
        meta.insert("view".into(), serde_json::json!("term"));
        meta.insert("edit".into(), serde_json::json!(true));

        assert_eq!(meta_get_string(&meta, "view", "default"), "term");
        assert_eq!(meta_get_string(&meta, "missing", "default"), "default");
        assert!(meta_get_bool(&meta, "edit", false));
        assert!(!meta_get_bool(&meta, "missing", false));
    }
}
