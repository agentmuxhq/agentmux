// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Suggestion types for autocomplete/search UI.
//! Port of Go's pkg/suggestion + wshrpc suggestion types.
//!
//! This module defines the wire types for file/bookmark suggestions
//! and a scoring/ranking engine. The actual directory listing and
//! S3 integration are deferred to later phases.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

// ---- Constants ----

/// Maximum suggestions returned per query.
pub const MAX_SUGGESTIONS: usize = 50;

/// Channel buffer size for directory listing results.
pub const LIST_DIR_CHAN_SIZE: usize = 50;

/// Maximum cached directory listing entries.
const MAX_CACHE_ENTRIES: usize = 20;

/// Cache TTL for directory listings.
const CACHE_TTL: Duration = Duration::from_secs(60);

// ---- Suggestion type constants ----

pub const SUGGESTION_TYPE_FILE: &str = "file";
pub const SUGGESTION_TYPE_URL: &str = "url";
pub const SUGGESTION_TYPE_BOOKMARK: &str = "bookmark";

// ---- Request/Response types (wire format) ----

/// Input data for fetching suggestions.
/// Matches Go's `wshrpc.FetchSuggestionsData` JSON tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FetchSuggestionsData {
    #[serde(rename = "suggestiontype", default)]
    pub suggestion_type: String,

    #[serde(default)]
    pub query: String,

    #[serde(rename = "widgetid", default)]
    pub widget_id: String,

    #[serde(rename = "reqnum", default)]
    pub req_num: i64,

    /// Current working directory (for file suggestions).
    #[serde(rename = "file:cwd", default, skip_serializing_if = "String::is_empty")]
    pub file_cwd: String,

    /// Only list directories.
    #[serde(rename = "file:dironly", default)]
    pub file_dir_only: bool,

    /// Connection string (e.g. "aws:region").
    #[serde(
        rename = "file:connection",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub file_connection: String,
}

/// Response containing matched suggestions.
/// Matches Go's `wshrpc.FetchSuggestionsResponse`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FetchSuggestionsResponse {
    #[serde(rename = "reqnum", default)]
    pub req_num: i64,

    #[serde(default)]
    pub suggestions: Vec<SuggestionType>,
}

/// A single suggestion item.
/// Matches Go's `wshrpc.SuggestionType` JSON tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestionType {
    /// "file" or "url".
    #[serde(rename = "type", default)]
    pub suggestion_type: String,

    /// Hash-based unique ID.
    #[serde(rename = "suggestionid", default)]
    pub suggestion_id: String,

    /// Main display text.
    #[serde(default)]
    pub display: String,

    /// Secondary text.
    #[serde(rename = "subtext", default, skip_serializing_if = "String::is_empty")]
    pub sub_text: String,

    /// Icon name/class.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,

    /// Icon color.
    #[serde(rename = "iconcolor", default, skip_serializing_if = "String::is_empty")]
    pub icon_color: String,

    /// Icon image source (e.g. favicon URL).
    #[serde(rename = "iconsrc", default, skip_serializing_if = "String::is_empty")]
    pub icon_src: String,

    /// Positions of matched characters in `display`.
    #[serde(rename = "matchpos", default, skip_serializing_if = "Vec::is_empty")]
    pub match_pos: Vec<i64>,

    /// Positions of matched characters in `sub_text`.
    #[serde(rename = "submatchpos", default, skip_serializing_if = "Vec::is_empty")]
    pub sub_match_pos: Vec<i64>,

    /// Fuzzy match score.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub score: i64,

    // -- File-specific fields --
    /// MIME type for files.
    #[serde(
        rename = "file:mimetype",
        default,
        skip_serializing_if = "String::is_empty"
    )]
    pub file_mime_type: String,

    /// Full filesystem path.
    #[serde(rename = "file:path", default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,

    /// Display filename.
    #[serde(rename = "file:name", default, skip_serializing_if = "String::is_empty")]
    pub file_name: String,

    // -- URL-specific fields --
    /// Full URL.
    #[serde(rename = "url:url", default, skip_serializing_if = "String::is_empty")]
    pub url_url: String,
}

fn is_zero(v: &i64) -> bool {
    *v == 0
}

// ---- Query resolution ----

/// Resolved file query components.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFileQuery {
    /// Base directory to list.
    pub base_dir: String,
    /// Prefix to prepend to filenames in display.
    pub query_prefix: String,
    /// Search term for fuzzy matching.
    pub search_term: String,
}

/// Parse a file suggestion query into base directory and search term.
///
/// Rules:
/// - Empty query → list cwd
/// - Trailing slash → list that directory
/// - Slashes present → split into dir + search
/// - No slashes → search in cwd
/// - `~` → home directory
pub fn resolve_file_query(query: &str, cwd: &str, home_dir: &str) -> ResolvedFileQuery {
    let expanded = if let Some(rest) = query.strip_prefix('~') {
        format!("{}{}", home_dir.trim_end_matches('/'), rest)
    } else {
        query.to_string()
    };

    if expanded.is_empty() {
        return ResolvedFileQuery {
            base_dir: cwd.to_string(),
            query_prefix: String::new(),
            search_term: String::new(),
        };
    }

    if expanded.ends_with('/') {
        return ResolvedFileQuery {
            base_dir: expanded.clone(),
            query_prefix: expanded,
            search_term: String::new(),
        };
    }

    if let Some(last_slash) = expanded.rfind('/') {
        let dir = &expanded[..=last_slash];
        let search = &expanded[last_slash + 1..];
        ResolvedFileQuery {
            base_dir: dir.to_string(),
            query_prefix: dir.to_string(),
            search_term: search.to_string(),
        }
    } else {
        ResolvedFileQuery {
            base_dir: cwd.to_string(),
            query_prefix: String::new(),
            search_term: expanded,
        }
    }
}

// ---- Fuzzy matching (simplified) ----

/// Fuzzy match result.
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub score: i64,
    pub positions: Vec<i64>,
}

/// Simple fuzzy match: checks if all characters of `pattern` appear
/// in `text` in order. Returns score based on consecutive matches,
/// prefix bonus, and case-exact bonus.
pub fn fuzzy_match(pattern: &str, text: &str) -> Option<FuzzyMatch> {
    if pattern.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            positions: vec![],
        });
    }

    let pattern_lower: Vec<char> = pattern.to_lowercase().chars().collect();
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    let mut positions = Vec::with_capacity(pattern_lower.len());
    let mut text_idx = 0;

    for &pc in &pattern_lower {
        let mut found = false;
        while text_idx < text_lower.len() {
            if text_lower[text_idx] == pc {
                positions.push(text_idx as i64);
                text_idx += 1;
                found = true;
                break;
            }
            text_idx += 1;
        }
        if !found {
            return None;
        }
    }

    // Score: base + consecutive bonus + prefix bonus + case bonus
    let mut score: i64 = 100;

    // Consecutive character bonus
    for i in 1..positions.len() {
        if positions[i] == positions[i - 1] + 1 {
            score += 20;
        }
    }

    // Prefix match bonus
    if !positions.is_empty() && positions[0] == 0 {
        score += 50;
    }

    // Case-exact bonus
    let pattern_chars: Vec<char> = pattern.chars().collect();
    for (i, &pos) in positions.iter().enumerate() {
        if i < pattern_chars.len() && text_chars[pos as usize] == pattern_chars[i] {
            score += 5;
        }
    }

    // Shorter match span is better
    if positions.len() >= 2 {
        let span = positions.last().unwrap() - positions[0];
        let min_span = positions.len() as i64 - 1;
        score -= (span - min_span) * 3;
    }

    Some(FuzzyMatch { score, positions })
}

// ---- Scored entry for top-k selection ----

/// Entry with its fuzzy match score, used for ranking.
#[derive(Debug, Clone)]
pub struct ScoredEntry {
    pub name: String,
    pub is_dir: bool,
    pub score: i64,
    pub positions: Vec<i64>,
}

/// Select the top-k entries by score, breaking ties by name length (shorter wins).
pub fn select_top_suggestions(entries: &mut Vec<ScoredEntry>, limit: usize) -> Vec<ScoredEntry> {
    entries.sort_by(|a, b| {
        b.score.cmp(&a.score).then_with(|| a.name.len().cmp(&b.name.len()))
    });
    entries.truncate(limit);
    entries.clone()
}

// ---- Suggestion ID generation ----

/// Generate a suggestion ID from the suggestion type and a key string.
/// Uses a simple hash for uniqueness.
pub fn make_suggestion_id(suggestion_type: &str, key: &str) -> String {
    // Simple hash: FNV-1a
    let mut hash: u64 = 14695981039346656037;
    for b in format!("{}:{}", suggestion_type, key).bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{:016x}", hash)
}

// ---- Directory listing cache ----

struct CacheEntry {
    key: String,
    entries: Vec<DirEntryResult>,
    expires: Instant,
}

/// Result from listing a directory entry.
#[derive(Debug, Clone)]
pub struct DirEntryResult {
    pub name: String,
    pub is_dir: bool,
    pub error: Option<String>,
}

/// LRU cache for directory listings, keyed by `widget_id|dir_path`.
pub struct SuggestionCache {
    entries: Mutex<Vec<CacheEntry>>,
}

impl SuggestionCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Look up cached directory listing.
    pub fn get(&self, key: &str) -> Option<Vec<DirEntryResult>> {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();

        // Find and validate
        let idx = entries.iter().position(|e| e.key == key)?;
        if entries[idx].expires <= now {
            entries.remove(idx);
            return None;
        }

        let result = entries[idx].entries.clone();

        // Move to front (LRU)
        let entry = entries.remove(idx);
        entries.insert(0, entry);

        Some(result)
    }

    /// Store a directory listing in the cache.
    pub fn set(&self, key: String, dir_entries: Vec<DirEntryResult>) {
        let mut entries = self.entries.lock().unwrap();

        // Remove existing entry with same key
        entries.retain(|e| e.key != key);

        // Insert at front
        entries.insert(
            0,
            CacheEntry {
                key,
                entries: dir_entries,
                expires: Instant::now() + CACHE_TTL,
            },
        );

        // Evict if over limit
        while entries.len() > MAX_CACHE_ENTRIES {
            entries.pop();
        }
    }

    /// Remove all cache entries for a widget.
    pub fn dispose(&self, widget_id: &str) {
        let mut entries = self.entries.lock().unwrap();
        let prefix = format!("{}|", widget_id);
        entries.retain(|e| !e.key.starts_with(&prefix));
    }

    /// Remove expired entries.
    pub fn clean(&self) {
        let mut entries = self.entries.lock().unwrap();
        let now = Instant::now();
        entries.retain(|e| e.expires > now);
    }

    /// Make a cache key from widget ID and directory path.
    pub fn make_key(widget_id: &str, dir_path: &str) -> String {
        format!("{}|{}", widget_id, dir_path)
    }

    /// Number of entries currently cached.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for SuggestionCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Build suggestion helpers ----

/// Build a file suggestion from a directory entry.
pub fn build_file_suggestion(
    name: &str,
    is_dir: bool,
    query_prefix: &str,
    base_dir: &str,
    fuzzy: Option<&FuzzyMatch>,
) -> SuggestionType {
    let display = if is_dir {
        format!("{}{}/", query_prefix, name)
    } else {
        format!("{}{}", query_prefix, name)
    };

    let full_path = format!(
        "{}/{}",
        base_dir.trim_end_matches('/'),
        name
    );

    let icon = if is_dir {
        "folder".to_string()
    } else {
        "file".to_string()
    };

    SuggestionType {
        suggestion_type: SUGGESTION_TYPE_FILE.to_string(),
        suggestion_id: make_suggestion_id(SUGGESTION_TYPE_FILE, &full_path),
        display,
        icon,
        file_path: full_path,
        file_name: name.to_string(),
        score: fuzzy.map_or(0, |f| f.score),
        match_pos: fuzzy.map_or(vec![], |f| f.positions.clone()),
        ..Default::default()
    }
}

/// Build a URL/bookmark suggestion.
pub fn build_url_suggestion(
    url: &str,
    title: &str,
    favicon_src: &str,
    fuzzy: Option<&FuzzyMatch>,
    sub_fuzzy: Option<&FuzzyMatch>,
) -> SuggestionType {
    SuggestionType {
        suggestion_type: SUGGESTION_TYPE_URL.to_string(),
        suggestion_id: make_suggestion_id(SUGGESTION_TYPE_URL, url),
        display: title.to_string(),
        sub_text: url.to_string(),
        icon_src: favicon_src.to_string(),
        url_url: url.to_string(),
        score: fuzzy.map_or(0, |f| f.score),
        match_pos: fuzzy.map_or(vec![], |f| f.positions.clone()),
        sub_match_pos: sub_fuzzy.map_or(vec![], |f| f.positions.clone()),
        ..Default::default()
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    // -- FetchSuggestionsData serde --

    #[test]
    fn test_fetch_suggestions_data_serde() {
        let data = FetchSuggestionsData {
            suggestion_type: "file".to_string(),
            query: "src/ma".to_string(),
            widget_id: "w-1".to_string(),
            req_num: 42,
            file_cwd: "/home/user".to_string(),
            file_dir_only: false,
            file_connection: String::new(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"suggestiontype\":\"file\""));
        assert!(json.contains("\"widgetid\":\"w-1\""));
        assert!(json.contains("\"reqnum\":42"));
        assert!(json.contains("\"file:cwd\":\"/home/user\""));
        // Empty connection should be omitted
        assert!(!json.contains("file:connection"));

        let parsed: FetchSuggestionsData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.suggestion_type, "file");
        assert_eq!(parsed.req_num, 42);
    }

    #[test]
    fn test_fetch_suggestions_data_from_go_json() {
        // Simulate JSON as Go would emit it
        let go_json = r#"{
            "suggestiontype": "file",
            "query": "test",
            "widgetid": "wgt-abc",
            "reqnum": 7,
            "file:cwd": "/tmp",
            "file:dironly": true,
            "file:connection": "aws:us-east-1"
        }"#;
        let parsed: FetchSuggestionsData = serde_json::from_str(go_json).unwrap();
        assert_eq!(parsed.suggestion_type, "file");
        assert_eq!(parsed.widget_id, "wgt-abc");
        assert!(parsed.file_dir_only);
        assert_eq!(parsed.file_connection, "aws:us-east-1");
    }

    // -- FetchSuggestionsResponse serde --

    #[test]
    fn test_fetch_suggestions_response_serde() {
        let resp = FetchSuggestionsResponse {
            req_num: 5,
            suggestions: vec![SuggestionType {
                suggestion_type: "file".to_string(),
                display: "main.rs".to_string(),
                file_path: "/src/main.rs".to_string(),
                ..Default::default()
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"reqnum\":5"));

        let parsed: FetchSuggestionsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.suggestions.len(), 1);
        assert_eq!(parsed.suggestions[0].display, "main.rs");
    }

    // -- SuggestionType serde --

    #[test]
    fn test_suggestion_type_serde_file() {
        let s = SuggestionType {
            suggestion_type: "file".to_string(),
            suggestion_id: "abc123".to_string(),
            display: "src/main.rs".to_string(),
            icon: "file".to_string(),
            file_mime_type: "text/x-rust".to_string(),
            file_path: "/project/src/main.rs".to_string(),
            file_name: "main.rs".to_string(),
            score: 150,
            match_pos: vec![0, 1, 2],
            ..Default::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"suggestionid\":\"abc123\""));
        assert!(json.contains("\"file:mimetype\":\"text/x-rust\""));
        assert!(json.contains("\"file:path\":\"/project/src/main.rs\""));
        assert!(json.contains("\"file:name\":\"main.rs\""));
        assert!(json.contains("\"matchpos\":[0,1,2]"));
        // Empty URL fields should be omitted
        assert!(!json.contains("url:url"));
        assert!(!json.contains("subtext"));
    }

    #[test]
    fn test_suggestion_type_serde_url() {
        let s = SuggestionType {
            suggestion_type: "url".to_string(),
            suggestion_id: "def456".to_string(),
            display: "Rust Docs".to_string(),
            sub_text: "https://doc.rust-lang.org".to_string(),
            icon_src: "https://doc.rust-lang.org/favicon.ico".to_string(),
            url_url: "https://doc.rust-lang.org".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"url:url\":\"https://doc.rust-lang.org\""));
        assert!(json.contains("\"subtext\":\"https://doc.rust-lang.org\""));
        assert!(json.contains("\"iconsrc\":\"https://doc.rust-lang.org/favicon.ico\""));
        // Empty file fields should be omitted
        assert!(!json.contains("file:path"));
        assert!(!json.contains("file:name"));
    }

    #[test]
    fn test_suggestion_type_from_go_json() {
        let go_json = r#"{
            "type": "file",
            "suggestionid": "x1",
            "display": "config.toml",
            "icon": "file",
            "score": 200,
            "matchpos": [0, 1, 2, 3],
            "file:mimetype": "application/toml",
            "file:path": "/etc/config.toml",
            "file:name": "config.toml"
        }"#;
        let parsed: SuggestionType = serde_json::from_str(go_json).unwrap();
        assert_eq!(parsed.suggestion_type, "file");
        assert_eq!(parsed.score, 200);
        assert_eq!(parsed.match_pos, vec![0, 1, 2, 3]);
        assert_eq!(parsed.file_mime_type, "application/toml");
    }

    // -- resolve_file_query --

    #[test]
    fn test_resolve_empty_query() {
        let r = resolve_file_query("", "/home/user", "/home/user");
        assert_eq!(r.base_dir, "/home/user");
        assert_eq!(r.search_term, "");
    }

    #[test]
    fn test_resolve_trailing_slash() {
        let r = resolve_file_query("src/", "/home/user", "/home/user");
        assert_eq!(r.base_dir, "src/");
        assert_eq!(r.query_prefix, "src/");
        assert_eq!(r.search_term, "");
    }

    #[test]
    fn test_resolve_with_search() {
        let r = resolve_file_query("src/ma", "/home/user", "/home/user");
        assert_eq!(r.base_dir, "src/");
        assert_eq!(r.query_prefix, "src/");
        assert_eq!(r.search_term, "ma");
    }

    #[test]
    fn test_resolve_no_slash() {
        let r = resolve_file_query("main", "/home/user", "/home/user");
        assert_eq!(r.base_dir, "/home/user");
        assert_eq!(r.search_term, "main");
    }

    #[test]
    fn test_resolve_tilde() {
        let r = resolve_file_query("~/docs/", "/tmp", "/home/user");
        assert_eq!(r.base_dir, "/home/user/docs/");
        assert_eq!(r.search_term, "");
    }

    #[test]
    fn test_resolve_tilde_with_search() {
        let r = resolve_file_query("~/re", "/tmp", "/home/user");
        assert_eq!(r.base_dir, "/home/user/");
        assert_eq!(r.search_term, "re");
    }

    #[test]
    fn test_resolve_absolute_path() {
        let r = resolve_file_query("/etc/hos", "/home", "/home");
        assert_eq!(r.base_dir, "/etc/");
        assert_eq!(r.search_term, "hos");
    }

    // -- fuzzy_match --

    #[test]
    fn test_fuzzy_match_exact() {
        let m = fuzzy_match("main", "main.rs").unwrap();
        assert!(m.score > 100);
        assert_eq!(m.positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_partial() {
        let m = fuzzy_match("mr", "main.rs").unwrap();
        assert!(m.score > 0);
        assert_eq!(m.positions.len(), 2);
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        let m = fuzzy_match("MAIN", "main.rs").unwrap();
        assert!(m.score > 0);
        assert_eq!(m.positions, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        assert!(fuzzy_match("xyz", "main.rs").is_none());
    }

    #[test]
    fn test_fuzzy_match_empty_pattern() {
        let m = fuzzy_match("", "anything").unwrap();
        assert_eq!(m.score, 0);
        assert!(m.positions.is_empty());
    }

    #[test]
    fn test_fuzzy_match_prefix_bonus() {
        let prefix = fuzzy_match("ma", "main.rs").unwrap();
        let middle = fuzzy_match("in", "main.rs").unwrap();
        assert!(prefix.score > middle.score);
    }

    // -- select_top_suggestions --

    #[test]
    fn test_select_top_suggestions() {
        let mut entries = vec![
            ScoredEntry {
                name: "low.txt".into(),
                is_dir: false,
                score: 50,
                positions: vec![],
            },
            ScoredEntry {
                name: "high.txt".into(),
                is_dir: false,
                score: 200,
                positions: vec![],
            },
            ScoredEntry {
                name: "med.txt".into(),
                is_dir: false,
                score: 100,
                positions: vec![],
            },
        ];
        let top = select_top_suggestions(&mut entries, 2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].name, "high.txt");
        assert_eq!(top[1].name, "med.txt");
    }

    #[test]
    fn test_select_top_tie_breaking() {
        let mut entries = vec![
            ScoredEntry {
                name: "longer_name.txt".into(),
                is_dir: false,
                score: 100,
                positions: vec![],
            },
            ScoredEntry {
                name: "short.txt".into(),
                is_dir: false,
                score: 100,
                positions: vec![],
            },
        ];
        let top = select_top_suggestions(&mut entries, 2);
        // Shorter name wins on tie
        assert_eq!(top[0].name, "short.txt");
    }

    // -- make_suggestion_id --

    #[test]
    fn test_make_suggestion_id_deterministic() {
        let id1 = make_suggestion_id("file", "/src/main.rs");
        let id2 = make_suggestion_id("file", "/src/main.rs");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_make_suggestion_id_different_types() {
        let file_id = make_suggestion_id("file", "test");
        let url_id = make_suggestion_id("url", "test");
        assert_ne!(file_id, url_id);
    }

    // -- SuggestionCache --

    #[test]
    fn test_cache_set_get() {
        let cache = SuggestionCache::new();
        let entries = vec![DirEntryResult {
            name: "test.txt".into(),
            is_dir: false,
            error: None,
        }];
        cache.set("w1|/tmp".into(), entries);
        let result = cache.get("w1|/tmp");
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = SuggestionCache::new();
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_dispose() {
        let cache = SuggestionCache::new();
        cache.set("w1|/tmp".into(), vec![]);
        cache.set("w1|/home".into(), vec![]);
        cache.set("w2|/tmp".into(), vec![]);
        assert_eq!(cache.len(), 3);

        cache.dispose("w1");
        assert_eq!(cache.len(), 1);
        assert!(cache.get("w1|/tmp").is_none());
        assert!(cache.get("w2|/tmp").is_some());
    }

    #[test]
    fn test_cache_eviction() {
        let cache = SuggestionCache::new();
        for i in 0..MAX_CACHE_ENTRIES + 5 {
            cache.set(format!("w|{}", i), vec![]);
        }
        assert_eq!(cache.len(), MAX_CACHE_ENTRIES);
    }

    #[test]
    fn test_cache_make_key() {
        assert_eq!(SuggestionCache::make_key("w1", "/tmp"), "w1|/tmp");
    }

    // -- build_file_suggestion --

    #[test]
    fn test_build_file_suggestion() {
        let s = build_file_suggestion("main.rs", false, "src/", "/project/src", None);
        assert_eq!(s.display, "src/main.rs");
        assert_eq!(s.file_path, "/project/src/main.rs");
        assert_eq!(s.icon, "file");
        assert_eq!(s.suggestion_type, "file");
    }

    #[test]
    fn test_build_dir_suggestion() {
        let s = build_file_suggestion("docs", true, "", "/project", None);
        assert_eq!(s.display, "docs/");
        assert_eq!(s.icon, "folder");
    }

    #[test]
    fn test_build_file_with_score() {
        let fuzzy = FuzzyMatch {
            score: 150,
            positions: vec![0, 1],
        };
        let s = build_file_suggestion("main.rs", false, "", "/src", Some(&fuzzy));
        assert_eq!(s.score, 150);
        assert_eq!(s.match_pos, vec![0, 1]);
    }

    // -- build_url_suggestion --

    #[test]
    fn test_build_url_suggestion() {
        let s = build_url_suggestion(
            "https://example.com",
            "Example",
            "https://example.com/favicon.ico",
            None,
            None,
        );
        assert_eq!(s.suggestion_type, "url");
        assert_eq!(s.display, "Example");
        assert_eq!(s.sub_text, "https://example.com");
        assert_eq!(s.url_url, "https://example.com");
        assert_eq!(s.icon_src, "https://example.com/favicon.ico");
    }
}
