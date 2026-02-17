// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Favicon cache: in-memory caching of website favicons.
//! Port of Go's pkg/faviconcache/.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---- Constants ----

/// How long cached favicons remain valid.
pub const CACHE_DURATION: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Maximum favicon size in bytes.
pub const MAX_ICON_SIZE: usize = 256 * 1024; // 256KB

/// Maximum number of concurrent fetch operations.
pub const MAX_CONCURRENT_FETCHES: usize = 5;

// ---- Types ----

/// A cached favicon entry.
#[derive(Debug, Clone)]
pub struct FaviconCacheItem {
    /// Base64-encoded data URL (e.g., "data:image/png;base64,...").
    pub data: String,
    /// When this entry was last fetched.
    pub last_fetched: Instant,
}

impl FaviconCacheItem {
    /// Check if this cache entry has expired.
    pub fn is_expired(&self) -> bool {
        self.last_fetched.elapsed() > CACHE_DURATION
    }
}

/// In-memory favicon cache with domain-keyed entries.
pub struct FaviconCache {
    entries: Mutex<HashMap<String, FaviconCacheItem>>,
    /// Domains currently being fetched (to prevent duplicate fetches).
    in_progress: Mutex<HashMap<String, bool>>,
}

impl Default for FaviconCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FaviconCache {
    /// Create a new empty favicon cache.
    pub fn new() -> Self {
        FaviconCache {
            entries: Mutex::new(HashMap::new()),
            in_progress: Mutex::new(HashMap::new()),
        }
    }

    /// Get a favicon from the cache by domain.
    /// Returns None if not cached or expired.
    pub fn get(&self, domain: &str) -> Option<FaviconCacheItem> {
        let entries = self.entries.lock().unwrap();
        entries.get(domain).and_then(|item| {
            if item.is_expired() {
                None
            } else {
                Some(item.clone())
            }
        })
    }

    /// Store a favicon in the cache.
    pub fn set(&self, domain: &str, data: String) {
        let mut entries = self.entries.lock().unwrap();
        entries.insert(
            domain.to_string(),
            FaviconCacheItem {
                data,
                last_fetched: Instant::now(),
            },
        );
    }

    /// Check if a domain is currently being fetched.
    pub fn is_fetching(&self, domain: &str) -> bool {
        let in_progress = self.in_progress.lock().unwrap();
        *in_progress.get(domain).unwrap_or(&false)
    }

    /// Mark a domain as being fetched. Returns false if already in progress.
    pub fn start_fetch(&self, domain: &str) -> bool {
        let mut in_progress = self.in_progress.lock().unwrap();
        if *in_progress.get(domain).unwrap_or(&false) {
            return false;
        }
        in_progress.insert(domain.to_string(), true);
        true
    }

    /// Mark a domain fetch as complete.
    pub fn finish_fetch(&self, domain: &str) {
        let mut in_progress = self.in_progress.lock().unwrap();
        in_progress.remove(domain);
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.lock().unwrap().is_empty()
    }

    /// Remove expired entries from the cache.
    pub fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|_, item| !item.is_expired());
        before - entries.len()
    }

    /// Extract the domain from a URL string.
    pub fn extract_domain(url: &str) -> Option<String> {
        // Strip scheme
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);

        // Take everything up to the first / or end
        let domain = without_scheme.split('/').next()?;

        // Strip port
        let domain = domain.split(':').next()?;

        if domain.is_empty() {
            None
        } else {
            Some(domain.to_lowercase())
        }
    }

    /// Get the favicon URL for a domain.
    /// Special-cases known domains with non-standard favicon paths.
    pub fn get_favicon_url(domain: &str) -> String {
        // GitHub uses a non-standard favicon path
        if domain == "github.com" {
            return "https://github.githubassets.com/favicons/favicon-dark.png".to_string();
        }
        format!("https://{}/favicon.ico", domain)
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(CACHE_DURATION, Duration::from_secs(86400));
        assert_eq!(MAX_ICON_SIZE, 262144);
        assert_eq!(MAX_CONCURRENT_FETCHES, 5);
    }

    #[test]
    fn test_cache_set_get() {
        let cache = FaviconCache::new();
        cache.set("example.com", "data:image/png;base64,abc123".to_string());

        let item = cache.get("example.com");
        assert!(item.is_some());
        assert_eq!(item.unwrap().data, "data:image/png;base64,abc123");
    }

    #[test]
    fn test_cache_miss() {
        let cache = FaviconCache::new();
        assert!(cache.get("nonexistent.com").is_none());
    }

    #[test]
    fn test_cache_overwrite() {
        let cache = FaviconCache::new();
        cache.set("example.com", "old-data".to_string());
        cache.set("example.com", "new-data".to_string());

        let item = cache.get("example.com").unwrap();
        assert_eq!(item.data, "new-data");
    }

    #[test]
    fn test_cache_len() {
        let cache = FaviconCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.set("a.com", "data-a".to_string());
        cache.set("b.com", "data-b".to_string());
        assert_eq!(cache.len(), 2);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_fetch_tracking() {
        let cache = FaviconCache::new();
        assert!(!cache.is_fetching("example.com"));

        // First start_fetch should succeed
        assert!(cache.start_fetch("example.com"));
        assert!(cache.is_fetching("example.com"));

        // Second start_fetch should fail (already in progress)
        assert!(!cache.start_fetch("example.com"));

        // Finish and verify
        cache.finish_fetch("example.com");
        assert!(!cache.is_fetching("example.com"));

        // Can start again after finish
        assert!(cache.start_fetch("example.com"));
    }

    #[test]
    fn test_extract_domain_https() {
        assert_eq!(
            FaviconCache::extract_domain("https://www.example.com/path"),
            Some("www.example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_http() {
        assert_eq!(
            FaviconCache::extract_domain("http://example.com"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_with_port() {
        assert_eq!(
            FaviconCache::extract_domain("https://example.com:8080/api"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_no_scheme() {
        assert_eq!(
            FaviconCache::extract_domain("example.com/path"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_empty() {
        assert_eq!(FaviconCache::extract_domain(""), None);
        assert_eq!(FaviconCache::extract_domain("https://"), None);
    }

    #[test]
    fn test_extract_domain_lowercase() {
        assert_eq!(
            FaviconCache::extract_domain("https://GitHub.Com/user/repo"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn test_get_favicon_url_standard() {
        let url = FaviconCache::get_favicon_url("example.com");
        assert_eq!(url, "https://example.com/favicon.ico");
    }

    #[test]
    fn test_get_favicon_url_github() {
        let url = FaviconCache::get_favicon_url("github.com");
        assert_eq!(
            url,
            "https://github.githubassets.com/favicons/favicon-dark.png"
        );
    }

    #[test]
    fn test_cache_item_not_expired() {
        let item = FaviconCacheItem {
            data: "test".to_string(),
            last_fetched: Instant::now(),
        };
        assert!(!item.is_expired());
    }

    #[test]
    fn test_default() {
        let cache = FaviconCache::default();
        assert!(cache.is_empty());
    }
}
