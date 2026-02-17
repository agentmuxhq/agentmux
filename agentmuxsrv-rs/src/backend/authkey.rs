// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Auth key management for WebSocket/HTTP request authentication.
//! Port of Go's pkg/authkey/.

use std::env;
use std::sync::OnceLock;
use subtle::ConstantTimeEq;

/// Environment variable name for the auth key.
pub const WAVE_AUTH_KEY_ENV: &str = "WAVETERM_AUTH_KEY";

/// HTTP header name for auth key.
pub const AUTH_KEY_HEADER: &str = "X-AuthKey";

/// Cached auth key (set once at startup).
static AUTH_KEY: OnceLock<String> = OnceLock::new();

/// Read the auth key from the environment variable and cache it.
/// Removes the env var after reading for security.
pub fn set_auth_key_from_env() -> Result<(), String> {
    let key = env::var(WAVE_AUTH_KEY_ENV)
        .map_err(|_| format!("{} not set", WAVE_AUTH_KEY_ENV))?;

    if key.is_empty() {
        return Err(format!("{} is empty", WAVE_AUTH_KEY_ENV));
    }

    let _ = AUTH_KEY.set(key);

    // Remove from environment for security
    env::remove_var(WAVE_AUTH_KEY_ENV);

    Ok(())
}

/// Get the cached auth key. Returns empty string if not set.
pub fn get_auth_key() -> &'static str {
    AUTH_KEY.get().map_or("", |k| k.as_str())
}

/// Validate an auth key value against the cached key.
/// Returns Ok(()) if the key matches, Err if missing or mismatched.
pub fn validate_auth_key(provided_key: &str) -> Result<(), String> {
    let expected = get_auth_key();
    if expected.is_empty() {
        return Err("auth key not configured".to_string());
    }
    if provided_key.is_empty() {
        return Err("no auth key provided".to_string());
    }
    if provided_key.as_bytes().ct_eq(expected.as_bytes()).into() {
        Ok(())
    } else {
        Err("invalid auth key".to_string())
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(WAVE_AUTH_KEY_ENV, "WAVETERM_AUTH_KEY");
        assert_eq!(AUTH_KEY_HEADER, "X-AuthKey");
    }

    #[test]
    fn test_get_auth_key_default() {
        // In a fresh test process, auth key should be empty or whatever was set
        let key = get_auth_key();
        // Just verify it doesn't panic
        let _ = key;
    }

    #[test]
    fn test_validate_auth_key_mismatch() {
        // With no key configured (or a different key set by other tests),
        // validating a random key should always fail
        let result = validate_auth_key("some-key");
        // Either "auth key not configured" or "invalid auth key" depending on test order
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_auth_key_empty_provided() {
        let result = validate_auth_key("");
        assert!(result.is_err());
    }
}
