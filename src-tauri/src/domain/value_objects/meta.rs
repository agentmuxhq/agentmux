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

    #[test]
    fn test_merge_meta_both_empty() {
        let base = MetaMapType::new();
        let update = MetaMapType::new();
        let result = merge_meta(&base, &update, true);
        assert!(result.is_empty());
    }

    #[test]
    fn test_merge_meta_empty_update() {
        let mut base = MetaMapType::new();
        base.insert("key".into(), serde_json::json!("value"));
        let update = MetaMapType::new();
        let result = merge_meta(&base, &update, true);
        assert_eq!(result.len(), 1);
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_merge_meta_section_clear_false_value() {
        let mut base = MetaMapType::new();
        base.insert("frame".into(), serde_json::json!(true));
        base.insert("frame:title".into(), serde_json::json!("hello"));

        let mut update = MetaMapType::new();
        update.insert("frame:*".into(), serde_json::json!(false));

        let result = merge_meta(&base, &update, true);
        // false value should NOT trigger section clear
        assert!(result.contains_key("frame"));
        assert!(result.contains_key("frame:title"));
    }

    #[test]
    fn test_merge_meta_section_clear_non_bool() {
        let mut base = MetaMapType::new();
        base.insert("frame".into(), serde_json::json!(true));
        base.insert("frame:title".into(), serde_json::json!("hello"));

        let mut update = MetaMapType::new();
        update.insert("frame:*".into(), serde_json::json!("yes"));

        let result = merge_meta(&base, &update, true);
        // non-bool value should NOT trigger section clear
        assert!(result.contains_key("frame"));
    }

    #[test]
    fn test_merge_meta_display_included_when_special() {
        let base = MetaMapType::new();
        let mut update = MetaMapType::new();
        update.insert("display:name".into(), serde_json::json!("test"));

        let result = merge_meta(&base, &update, true);
        assert!(result.contains_key("display:name"));
    }

    #[test]
    fn test_merge_meta_overwrite_preserves_other_keys() {
        let mut base = MetaMapType::new();
        base.insert("a".into(), serde_json::json!(1));
        base.insert("b".into(), serde_json::json!(2));

        let mut update = MetaMapType::new();
        update.insert("a".into(), serde_json::json!(10));

        let result = merge_meta(&base, &update, true);
        assert_eq!(result["a"], 10);
        assert_eq!(result["b"], 2);
    }

    #[test]
    fn test_meta_get_string_with_non_string_value() {
        let mut meta = MetaMapType::new();
        meta.insert("count".into(), serde_json::json!(42));
        assert_eq!(meta_get_string(&meta, "count", "default"), "default");
    }

    #[test]
    fn test_meta_get_bool_with_non_bool_value() {
        let mut meta = MetaMapType::new();
        meta.insert("name".into(), serde_json::json!("hello"));
        assert!(!meta_get_bool(&meta, "name", false));
        assert!(meta_get_bool(&meta, "name", true));
    }

    #[test]
    fn test_wave_obj_update_serde() {
        let update = WaveObjUpdate {
            updatetype: UPDATE_TYPE_UPDATE.to_string(),
            otype: "tab".to_string(),
            oid: "t-123".to_string(),
            obj: Some(serde_json::json!({"name": "Shell"})),
        };
        let json = serde_json::to_string(&update).unwrap();
        let parsed: WaveObjUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.updatetype, "update");
        assert_eq!(parsed.otype, "tab");
        assert!(parsed.obj.is_some());
    }

    #[test]
    fn test_wave_obj_update_delete_no_obj() {
        let update = WaveObjUpdate {
            updatetype: UPDATE_TYPE_DELETE.to_string(),
            otype: "block".to_string(),
            oid: "b-123".to_string(),
            obj: None,
        };
        let json = serde_json::to_string(&update).unwrap();
        assert!(!json.contains("obj"));
        let parsed: WaveObjUpdate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.updatetype, "delete");
        assert!(parsed.obj.is_none());
    }

    #[test]
    fn test_update_type_constants() {
        assert_eq!(UPDATE_TYPE_UPDATE, "update");
        assert_eq!(UPDATE_TYPE_DELETE, "delete");
    }
}
