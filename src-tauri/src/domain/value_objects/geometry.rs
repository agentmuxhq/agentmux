// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

//! Geometric value objects: Point, WinSize, TermSize, RuntimeOpts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WinSize {
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TermSize {
    pub rows: i64,
    pub cols: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeOpts {
    #[serde(default, skip_serializing_if = "is_default_term_size")]
    pub termsize: TermSize,
    #[serde(default, skip_serializing_if = "is_default_win_size")]
    pub winsize: WinSize,
}

fn is_default_term_size(ts: &TermSize) -> bool {
    ts.rows == 0 && ts.cols == 0
}
fn is_default_win_size(ws: &WinSize) -> bool {
    ws.width == 0 && ws.height == 0
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIContext {
    #[serde(rename = "windowid")]
    pub window_id: String,
    #[serde(rename = "activetabid")]
    pub active_tab_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
    }

    #[test]
    fn test_winsize_serde() {
        let ws = WinSize { width: 1920, height: 1080 };
        let json = serde_json::to_string(&ws).unwrap();
        let parsed: WinSize = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ws);
    }

    #[test]
    fn test_termsize_serde() {
        let ts = TermSize { rows: 24, cols: 80 };
        let json = serde_json::to_string(&ts).unwrap();
        let parsed: TermSize = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn test_runtime_opts_skip_default() {
        let opts = RuntimeOpts::default();
        let json = serde_json::to_string(&opts).unwrap();
        // Default sizes should be skipped
        assert!(!json.contains("termsize"));
        assert!(!json.contains("winsize"));
    }

    #[test]
    fn test_runtime_opts_includes_nondefault() {
        let opts = RuntimeOpts {
            termsize: TermSize { rows: 24, cols: 80 },
            winsize: WinSize { width: 800, height: 600 },
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("termsize"));
        assert!(json.contains("winsize"));
        let parsed: RuntimeOpts = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.termsize.rows, 24);
        assert_eq!(parsed.winsize.width, 800);
    }

    #[test]
    fn test_point_serde_roundtrip() {
        let p = Point { x: -100, y: 200 };
        let json = serde_json::to_string(&p).unwrap();
        let parsed: Point = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }

    #[test]
    fn test_point_equality() {
        assert_eq!(Point { x: 1, y: 2 }, Point { x: 1, y: 2 });
        assert_ne!(Point { x: 1, y: 2 }, Point { x: 1, y: 3 });
    }

    #[test]
    fn test_termsize_equality() {
        assert_eq!(TermSize { rows: 24, cols: 80 }, TermSize { rows: 24, cols: 80 });
        assert_ne!(TermSize { rows: 24, cols: 80 }, TermSize { rows: 25, cols: 80 });
    }

    #[test]
    fn test_ui_context_serde() {
        let ctx = UIContext {
            window_id: "win-1".into(),
            active_tab_id: "tab-1".into(),
        };
        let json = serde_json::to_string(&ctx).unwrap();
        // Verify rename fields
        assert!(json.contains("windowid"));
        assert!(json.contains("activetabid"));
        assert!(!json.contains("window_id"));

        let parsed: UIContext = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.window_id, "win-1");
        assert_eq!(parsed.active_tab_id, "tab-1");
    }

    #[test]
    fn test_winsize_default() {
        let ws = WinSize::default();
        assert_eq!(ws.width, 0);
        assert_eq!(ws.height, 0);
    }

    #[test]
    fn test_termsize_default() {
        let ts = TermSize::default();
        assert_eq!(ts.rows, 0);
        assert_eq!(ts.cols, 0);
    }
}
