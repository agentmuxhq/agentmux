// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! WaveObj entities: Rust equivalents of Go structs from pkg/waveobj/wtype.go.
//! All `#[serde(rename = "...")]` tags match Go JSON tags for wire compatibility.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::value_objects::{
    MetaMapType, ORef, Point, RuntimeOpts, WinSize,
    OTYPE_BLOCK, OTYPE_CLIENT, OTYPE_LAYOUT, OTYPE_TAB, OTYPE_WINDOW, OTYPE_WORKSPACE,
};

// ---- WaveObj trait ----

/// Rust equivalent of Go's `WaveObj` interface.
/// Every wave object has an otype, an OID, a version, and metadata.
pub trait WaveObj: Serialize + for<'de> Deserialize<'de> {
    fn get_otype() -> &'static str;
    fn get_oid(&self) -> &str;
    fn set_oid(&mut self, oid: String);
    fn get_version(&self) -> i64;
    fn set_version(&mut self, version: i64);
    fn get_meta(&self) -> &MetaMapType;
    fn set_meta(&mut self, meta: MetaMapType);

    fn oref(&self) -> ORef {
        ORef::new(Self::get_otype(), self.get_oid())
    }
}

/// Macro that implements `WaveObj` for a struct with standard fields.
macro_rules! impl_wave_obj {
    ($ty:ty, $otype:expr) => {
        impl WaveObj for $ty {
            fn get_otype() -> &'static str {
                $otype
            }
            fn get_oid(&self) -> &str {
                &self.oid
            }
            fn set_oid(&mut self, oid: String) {
                self.oid = oid;
            }
            fn get_version(&self) -> i64 {
                self.version
            }
            fn set_version(&mut self, version: i64) {
                self.version = version;
            }
            fn get_meta(&self) -> &MetaMapType {
                &self.meta
            }
            fn set_meta(&mut self, meta: MetaMapType) {
                self.meta = meta;
            }
        }
    };
}

// ---- Supporting types ----

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileDef {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<HashMap<String, FileDef>>,
    #[serde(default, skip_serializing_if = "MetaMapType::is_empty")]
    pub meta: MetaMapType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerClickOpts {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sendinput: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub createblock: Option<BlockDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerDisplayOpts {
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub imgsrc: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub svgblob: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerType {
    pub stickertype: String,
    pub style: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clickopts: Option<StickerClickOpts>,
    pub display: StickerDisplayOpts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutActionData {
    pub actiontype: String,
    pub actionid: String,
    pub blockid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodesize: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexarr: Option<Vec<i32>>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub magnified: bool,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub targetblockid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub position: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeafOrderEntry {
    pub nodeid: String,
    pub blockid: String,
}

// ====================================================================
// Core WaveObj entities
// ====================================================================

/// Client entity — represents a Wave client instance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Client {
    pub oid: String,
    pub version: i64,
    #[serde(default)]
    pub windowids: Vec<String>,
    #[serde(default)]
    pub meta: MetaMapType,
    #[serde(default, skip_serializing_if = "is_zero_i64")]
    pub tosagreed: i64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hasoldhistory: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tempoid: String,
}

impl_wave_obj!(Client, OTYPE_CLIENT);

/// Window entity — represents a desktop window.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Window {
    pub oid: String,
    pub version: i64,
    #[serde(default)]
    pub workspaceid: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub isnew: bool,
    #[serde(default)]
    pub pos: Point,
    #[serde(default)]
    pub winsize: WinSize,
    #[serde(default)]
    pub lastfocusts: i64,
    #[serde(default)]
    pub meta: MetaMapType,
}

impl_wave_obj!(Window, OTYPE_WINDOW);

/// Workspace entity — groups tabs together.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub oid: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
    #[serde(default)]
    pub tabids: Vec<String>,
    #[serde(default)]
    pub pinnedtabids: Vec<String>,
    #[serde(default)]
    pub activetabid: String,
    #[serde(default)]
    pub meta: MetaMapType,
}

impl_wave_obj!(Workspace, OTYPE_WORKSPACE);

/// Tab entity — contains blocks arranged by a layout.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tab {
    pub oid: String,
    pub version: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub layoutstate: String,
    #[serde(default)]
    pub blockids: Vec<String>,
    #[serde(default)]
    pub meta: MetaMapType,
}

impl_wave_obj!(Tab, OTYPE_TAB);

/// LayoutState entity — tree-based layout for a tab's blocks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutState {
    pub oid: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rootnode: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub magnifiednodeid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub focusednodeid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leaforder: Option<Vec<LeafOrderEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pendingbackendactions: Option<Vec<LayoutActionData>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<MetaMapType>,
}

// LayoutState has meta as Option<MetaMapType> (Go uses omitempty), so manual impl:
impl WaveObj for LayoutState {
    fn get_otype() -> &'static str {
        OTYPE_LAYOUT
    }
    fn get_oid(&self) -> &str {
        &self.oid
    }
    fn set_oid(&mut self, oid: String) {
        self.oid = oid;
    }
    fn get_version(&self) -> i64 {
        self.version
    }
    fn set_version(&mut self, version: i64) {
        self.version = version;
    }
    fn get_meta(&self) -> &MetaMapType {
        static EMPTY: std::sync::LazyLock<MetaMapType> =
            std::sync::LazyLock::new(MetaMapType::new);
        self.meta.as_ref().unwrap_or(&EMPTY)
    }
    fn set_meta(&mut self, meta: MetaMapType) {
        self.meta = Some(meta);
    }
}

/// Block entity — a terminal pane, editor, preview, or widget.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Block {
    pub oid: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parentoref: String,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtimeopts: Option<RuntimeOpts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stickers: Option<Vec<StickerType>>,
    #[serde(default)]
    pub meta: MetaMapType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subblockids: Option<Vec<String>>,
}

impl_wave_obj!(Block, OTYPE_BLOCK);

// ---- Serialization helpers ----

fn is_zero_i64(v: &i64) -> bool {
    *v == 0
}

/// Serialize any WaveObj to JSON bytes, including the "otype" field.
pub fn wave_obj_to_json<T: WaveObj>(obj: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut map = serde_json::to_value(obj)?;
    if let Some(m) = map.as_object_mut() {
        m.insert(
            "otype".to_string(),
            serde_json::Value::String(T::get_otype().to_string()),
        );
    }
    serde_json::to_vec(&map)
}

/// Deserialize JSON bytes to a specific WaveObj type.
pub fn wave_obj_from_json<T: WaveObj>(data: &[u8]) -> Result<T, serde_json::Error> {
    serde_json::from_slice(data)
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_roundtrip() {
        let client = Client {
            oid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            version: 3,
            windowids: vec!["w1".to_string(), "w2".to_string()],
            meta: MetaMapType::new(),
            tosagreed: 1700000000000,
            ..Default::default()
        };
        let json = wave_obj_to_json(&client).unwrap();
        let parsed: Client = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.oid, client.oid);
        assert_eq!(parsed.version, client.version);
        assert_eq!(parsed.windowids, client.windowids);
    }

    #[test]
    fn test_window_roundtrip() {
        let window = Window {
            oid: "w-1".to_string(),
            version: 1,
            workspaceid: "ws-123".to_string(),
            pos: Point { x: 100, y: 200 },
            winsize: WinSize {
                width: 1920,
                height: 1080,
            },
            ..Default::default()
        };
        let json = wave_obj_to_json(&window).unwrap();
        let parsed: Window = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.workspaceid, "ws-123");
        assert_eq!(parsed.pos.x, 100);
    }

    #[test]
    fn test_workspace_roundtrip() {
        let ws = Workspace {
            oid: "ws-oid".to_string(),
            version: 2,
            name: "My Workspace".to_string(),
            tabids: vec!["t1".to_string(), "t2".to_string()],
            activetabid: "t1".to_string(),
            ..Default::default()
        };
        let json = wave_obj_to_json(&ws).unwrap();
        let parsed: Workspace = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.name, "My Workspace");
        assert_eq!(parsed.tabids.len(), 2);
    }

    #[test]
    fn test_tab_roundtrip() {
        let tab = Tab {
            oid: "tab-oid".to_string(),
            version: 1,
            name: "Tab 1".to_string(),
            layoutstate: "ls-123".to_string(),
            blockids: vec!["b1".to_string()],
            meta: MetaMapType::new(),
        };
        let json = wave_obj_to_json(&tab).unwrap();
        let parsed: Tab = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.name, "Tab 1");
    }

    #[test]
    fn test_block_roundtrip() {
        let block = Block {
            oid: "blk-oid".to_string(),
            version: 5,
            parentoref: "tab:parent-id".to_string(),
            meta: {
                let mut m = MetaMapType::new();
                m.insert("view".to_string(), serde_json::json!("term"));
                m
            },
            ..Default::default()
        };
        let json = wave_obj_to_json(&block).unwrap();
        let parsed: Block = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.parentoref, "tab:parent-id");
    }

    #[test]
    fn test_wave_obj_includes_otype() {
        let client = Client {
            oid: "test".to_string(),
            version: 1,
            ..Default::default()
        };
        let json_bytes = wave_obj_to_json(&client).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&json_bytes).unwrap();
        assert_eq!(v["otype"], "client");
    }

    #[test]
    fn test_wire_compat_go_json() {
        let go_json = r#"{"oid":"abc-123","otype":"client","version":2,"windowids":["w1"],"meta":{"view":"term"},"tosagreed":1700000000000}"#;
        let client: Client = serde_json::from_str(go_json).unwrap();
        assert_eq!(client.oid, "abc-123");
        assert_eq!(client.version, 2);
    }

    #[test]
    fn test_layout_state_roundtrip() {
        let ls = LayoutState {
            oid: "ls-oid".to_string(),
            version: 1,
            rootnode: Some(serde_json::json!({"type": "split"})),
            magnifiednodeid: "node-1".to_string(),
            ..Default::default()
        };
        let json = wave_obj_to_json(&ls).unwrap();
        let parsed: LayoutState = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.magnifiednodeid, "node-1");
    }

    // ---- WaveObj trait method tests ----

    #[test]
    fn test_wave_obj_trait_otype() {
        assert_eq!(Client::get_otype(), "client");
        assert_eq!(Window::get_otype(), "window");
        assert_eq!(Workspace::get_otype(), "workspace");
        assert_eq!(Tab::get_otype(), "tab");
        assert_eq!(LayoutState::get_otype(), "layout");
        assert_eq!(Block::get_otype(), "block");
    }

    #[test]
    fn test_wave_obj_trait_oid_accessors() {
        let mut client = Client {
            oid: "original".to_string(),
            ..Default::default()
        };
        assert_eq!(client.get_oid(), "original");

        client.set_oid("updated".to_string());
        assert_eq!(client.get_oid(), "updated");
    }

    #[test]
    fn test_wave_obj_trait_version_accessors() {
        let mut tab = Tab {
            version: 5,
            ..Default::default()
        };
        assert_eq!(tab.get_version(), 5);

        tab.set_version(10);
        assert_eq!(tab.get_version(), 10);
    }

    #[test]
    fn test_wave_obj_trait_meta_accessors() {
        let mut block = Block::default();
        assert!(block.get_meta().is_empty());

        let mut meta = MetaMapType::new();
        meta.insert("view".into(), serde_json::json!("term"));
        block.set_meta(meta);
        assert_eq!(block.get_meta().get("view").unwrap(), "term");
    }

    #[test]
    fn test_wave_obj_oref() {
        let ws = Workspace {
            oid: "ws-123".to_string(),
            ..Default::default()
        };
        let oref = ws.oref();
        assert_eq!(oref.otype, "workspace");
        assert_eq!(oref.oid, "ws-123");
        assert_eq!(oref.to_string(), "workspace:ws-123");
    }

    #[test]
    fn test_layout_state_meta_none_returns_empty() {
        let layout = LayoutState {
            meta: None,
            ..Default::default()
        };
        assert!(layout.get_meta().is_empty());
    }

    #[test]
    fn test_layout_state_set_meta() {
        let mut layout = LayoutState::default();
        let mut meta = MetaMapType::new();
        meta.insert("key".into(), serde_json::json!("value"));
        layout.set_meta(meta);
        assert!(layout.meta.is_some());
        assert_eq!(layout.get_meta().get("key").unwrap(), "value");
    }

    // ---- Default trait tests ----

    #[test]
    fn test_entity_defaults() {
        let client = Client::default();
        assert!(client.oid.is_empty());
        assert_eq!(client.version, 0);
        assert!(client.windowids.is_empty());
        assert!(!client.hasoldhistory);
        assert_eq!(client.tosagreed, 0);

        let window = Window::default();
        assert!(window.oid.is_empty());
        assert!(!window.isnew);
        assert_eq!(window.pos.x, 0);
        assert_eq!(window.winsize.width, 0);

        let tab = Tab::default();
        assert!(tab.blockids.is_empty());
        assert!(tab.layoutstate.is_empty());

        let block = Block::default();
        assert!(block.runtimeopts.is_none());
        assert!(block.stickers.is_none());
        assert!(block.subblockids.is_none());
    }

    // ---- Supporting types serde tests ----

    #[test]
    fn test_file_def_serde() {
        let fd = FileDef {
            content: "hello".into(),
            meta: Some({
                let mut m = HashMap::new();
                m.insert("type".into(), serde_json::json!("text"));
                m
            }),
        };
        let json = serde_json::to_string(&fd).unwrap();
        let parsed: FileDef = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, "hello");
        assert!(parsed.meta.is_some());
    }

    #[test]
    fn test_file_def_empty_skips_fields() {
        let fd = FileDef::default();
        let json = serde_json::to_string(&fd).unwrap();
        assert!(!json.contains("content"));
        assert!(!json.contains("meta"));
    }

    #[test]
    fn test_block_def_serde() {
        let bd = BlockDef {
            files: Some({
                let mut m = HashMap::new();
                m.insert("main.py".into(), FileDef { content: "print(1)".into(), meta: None });
                m
            }),
            meta: MetaMapType::new(),
        };
        let json = serde_json::to_string(&bd).unwrap();
        let parsed: BlockDef = serde_json::from_str(&json).unwrap();
        assert!(parsed.files.is_some());
        assert!(parsed.files.unwrap().contains_key("main.py"));
    }

    #[test]
    fn test_leaf_order_entry_serde() {
        let entry = LeafOrderEntry {
            nodeid: "n1".into(),
            blockid: "b1".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: LeafOrderEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.nodeid, "n1");
        assert_eq!(parsed.blockid, "b1");
    }

    #[test]
    fn test_layout_action_data_serde() {
        let action = LayoutActionData {
            actiontype: "insert".into(),
            actionid: "a1".into(),
            blockid: "b1".into(),
            nodesize: Some(50),
            indexarr: Some(vec![0, 1]),
            focused: true,
            magnified: false,
            ephemeral: false,
            targetblockid: String::new(),
            position: "right".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: LayoutActionData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.actiontype, "insert");
        assert_eq!(parsed.nodesize, Some(50));
        assert!(parsed.focused);
    }

    #[test]
    fn test_layout_state_with_leaforder() {
        let ls = LayoutState {
            oid: "ls-1".to_string(),
            version: 1,
            leaforder: Some(vec![
                LeafOrderEntry { nodeid: "n1".into(), blockid: "b1".into() },
                LeafOrderEntry { nodeid: "n2".into(), blockid: "b2".into() },
            ]),
            ..Default::default()
        };
        let json = wave_obj_to_json(&ls).unwrap();
        let parsed: LayoutState = wave_obj_from_json(&json).unwrap();
        assert_eq!(parsed.leaforder.unwrap().len(), 2);
    }

    #[test]
    fn test_block_with_stickers_and_runtimeopts() {
        use crate::domain::value_objects::{RuntimeOpts, TermSize, WinSize};

        let block = Block {
            oid: "blk-1".to_string(),
            version: 1,
            runtimeopts: Some(RuntimeOpts {
                termsize: TermSize { rows: 24, cols: 80 },
                winsize: WinSize { width: 800, height: 600 },
            }),
            stickers: Some(vec![StickerType {
                stickertype: "icon".into(),
                style: HashMap::new(),
                clickopts: None,
                display: StickerDisplayOpts {
                    icon: "terminal".into(),
                    imgsrc: String::new(),
                    svgblob: String::new(),
                },
            }]),
            ..Default::default()
        };
        let json = wave_obj_to_json(&block).unwrap();
        let parsed: Block = wave_obj_from_json(&json).unwrap();
        assert!(parsed.runtimeopts.is_some());
        assert_eq!(parsed.stickers.unwrap().len(), 1);
    }

    // ---- Serde skip_serializing_if tests ----

    #[test]
    fn test_client_skips_zero_tosagreed() {
        let client = Client {
            oid: "c1".into(),
            version: 1,
            ..Default::default()
        };
        let json = serde_json::to_string(&client).unwrap();
        assert!(!json.contains("tosagreed"));
    }

    #[test]
    fn test_client_includes_nonzero_tosagreed() {
        let client = Client {
            oid: "c1".into(),
            version: 1,
            tosagreed: 123456,
            ..Default::default()
        };
        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("tosagreed"));
    }

    #[test]
    fn test_window_skips_false_isnew() {
        let window = Window {
            oid: "w1".into(),
            version: 1,
            ..Default::default()
        };
        let json = serde_json::to_string(&window).unwrap();
        assert!(!json.contains("isnew"));
    }

    #[test]
    fn test_wave_obj_from_json_invalid_data() {
        let result = wave_obj_from_json::<Client>(b"not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_zero_i64_helper() {
        assert!(is_zero_i64(&0));
        assert!(!is_zero_i64(&1));
        assert!(!is_zero_i64(&-1));
    }
}
