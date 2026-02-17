// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shell utilities: quoting, detection, environment encoding.
//! Port of Go's pkg/util/shellutil/.

use base64::Engine;
use std::collections::HashMap;
use std::env;
use std::path::Path;

// ---- Constants ----

/// Maximum string length for quoting (10MB safety limit).
pub const MAX_QUOTE_SIZE: usize = 10_000_000;

pub const DEFAULT_TERM_TYPE: &str = "xterm-256color";
pub const DEFAULT_TERM_ROWS: u16 = 24;
pub const DEFAULT_TERM_COLS: u16 = 80;
pub const DEFAULT_SHELL_PATH: &str = "/bin/bash";

pub const SHELL_TYPE_BASH: &str = "bash";
pub const SHELL_TYPE_ZSH: &str = "zsh";
pub const SHELL_TYPE_FISH: &str = "fish";
pub const SHELL_TYPE_PWSH: &str = "pwsh";
pub const SHELL_TYPE_UNKNOWN: &str = "unknown";

pub const ZSH_INTEGRATION_DIR: &str = "shell/zsh";
pub const BASH_INTEGRATION_DIR: &str = "shell/bash";
pub const PWSH_INTEGRATION_DIR: &str = "shell/pwsh";
pub const FISH_INTEGRATION_DIR: &str = "shell/fish";
pub const WAVE_HOME_BIN_DIR: &str = "bin";

// ---- Quoting functions ----

/// Check if a string contains only safe characters that don't need quoting.
fn is_safe(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'/' || b == b'.' || b == b'-')
}

/// Validate an environment variable name (must match `^[A-Za-z_][A-Za-z0-9_]*$`).
pub fn is_valid_env_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let bytes = name.as_bytes();
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Check if string is within quote size limit.
fn check_quote_size(s: &str) -> bool {
    if s.len() > MAX_QUOTE_SIZE {
        tracing::warn!("string too long to quote: {} bytes", s.len());
        false
    } else {
        true
    }
}

/// Hard-quote a string for bash/zsh (prevents variable expansion).
///
/// Escapes `"`, `\`, `$`, `` ` ``, and newlines.
pub fn hard_quote(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    if is_safe(s) {
        return s.to_string();
    }
    if !check_quote_size(s) {
        return String::new();
    }

    let mut buf = String::with_capacity(s.len() + 5);
    buf.push('"');
    for b in s.bytes() {
        match b {
            b'"' | b'\\' | b'$' | b'`' => {
                buf.push('\\');
                buf.push(b as char);
            }
            b'\n' => {
                buf.push('\\');
                buf.push('\n');
            }
            _ => buf.push(b as char),
        }
    }
    buf.push('"');
    buf
}

/// Hard-quote a string for Fish shell (prevents variable expansion).
///
/// Does NOT escape newlines or backticks (Fish handles them differently).
pub fn hard_quote_fish(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    if is_safe(s) {
        return s.to_string();
    }
    if !check_quote_size(s) {
        return String::new();
    }

    let mut buf = String::with_capacity(s.len() + 5);
    buf.push('"');
    for b in s.bytes() {
        match b {
            b'"' | b'\\' | b'$' => {
                buf.push('\\');
                buf.push(b as char);
            }
            _ => buf.push(b as char),
        }
    }
    buf.push('"');
    buf
}

/// Hard-quote a string for PowerShell (prevents variable expansion).
///
/// Uses backtick (`` ` ``) as escape character. Uses `` `n `` for newlines.
pub fn hard_quote_powershell(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    if !check_quote_size(s) {
        return String::new();
    }

    let mut buf = String::with_capacity(s.len() + 5);
    buf.push('"');
    for b in s.bytes() {
        match b {
            b'"' | b'`' | b'$' => {
                buf.push('`');
                buf.push(b as char);
            }
            b'\n' => {
                buf.push('`');
                buf.push('n');
                buf.push(b as char);
            }
            _ => buf.push(b as char),
        }
    }
    buf.push('"');
    buf
}

/// Soft-quote a string for bash/zsh (allows variable expansion).
///
/// Special handling for tilde (`~`) paths: `~/safe` stays unquoted,
/// `~/with spaces` becomes `~"/with spaces"`.
pub fn soft_quote(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }

    // Handle tilde paths
    if let Some(rest) = s.strip_prefix('~') {
        if rest.is_empty() {
            return s.to_string();
        }
        if let Some(after_slash) = rest.strip_prefix('/') {
            // ~/safe/path → leave as-is
            if is_safe(after_slash) {
                return s.to_string();
            }
            // ~/path with special chars → ~"<quoted rest>"
            return format!("~{}", soft_quote(rest));
        }
    }

    if is_safe(s) {
        return s.to_string();
    }
    if !check_quote_size(s) {
        return String::new();
    }

    let mut buf = String::with_capacity(s.len() + 5);
    buf.push('"');
    for b in s.bytes() {
        // In soft quote, don't escape $ to allow variable expansion
        if b == b'"' || b == b'\\' || b == b'`' {
            buf.push('\\');
        }
        buf.push(b as char);
    }
    buf.push('"');
    buf
}

// ---- Shell detection ----

/// Detect the shell type from a shell path by examining the basename.
pub fn get_shell_type_from_path(shell_path: &str) -> &'static str {
    let basename = Path::new(shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if basename.contains("bash") {
        SHELL_TYPE_BASH
    } else if basename.contains("zsh") {
        SHELL_TYPE_ZSH
    } else if basename.contains("fish") {
        SHELL_TYPE_FISH
    } else if basename.contains("pwsh") || basename.contains("powershell") {
        SHELL_TYPE_PWSH
    } else {
        SHELL_TYPE_UNKNOWN
    }
}

/// Detect the local shell path from the environment.
pub fn detect_local_shell_path() -> String {
    if cfg!(target_os = "windows") {
        // On Windows, check common PowerShell paths
        for name in &["pwsh.exe", "pwsh", "powershell.exe", "powershell"] {
            if let Ok(path) = env::var("PATH") {
                for dir in env::split_paths(&path) {
                    let candidate = dir.join(name);
                    if candidate.exists() {
                        return candidate.to_string_lossy().to_string();
                    }
                }
            }
        }
        return "powershell.exe".to_string();
    }

    // On macOS/Linux, use SHELL env var
    if let Ok(shell) = env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }

    DEFAULT_SHELL_PATH.to_string()
}

/// Get the key portion of an "KEY=VALUE" environment string.
pub fn get_env_str_key(env_str: &str) -> &str {
    match env_str.find('=') {
        Some(idx) => &env_str[..idx],
        None => env_str,
    }
}

/// Format an OSC (Operating System Command) escape sequence.
pub fn format_osc(osc_num: u32, parts: &[&str]) -> String {
    if parts.is_empty() {
        format!("\x1b]{}\x07", osc_num)
    } else {
        format!("\x1b]{};{}\x07", osc_num, parts.join(";"))
    }
}

// ---- Environment encoding ----

/// Encode environment variables as bash/zsh export statements.
fn encode_env_vars_for_bash(env: &HashMap<String, String>) -> Result<String, String> {
    let mut encoded = String::new();
    for (k, v) in env {
        if !is_valid_env_var_name(k) {
            return Err(format!("invalid env var name: {:?}", k));
        }
        encoded.push_str(&format!("export {}={}\n", k, hard_quote(v)));
    }
    Ok(encoded)
}

/// Encode environment variables as fish shell set statements.
fn encode_env_vars_for_fish(env: &HashMap<String, String>) -> Result<String, String> {
    let mut encoded = String::new();
    for (k, v) in env {
        if !is_valid_env_var_name(k) {
            return Err(format!("invalid env var name: {:?}", k));
        }
        encoded.push_str(&format!("set -x {} {}\n", k, hard_quote_fish(v)));
    }
    Ok(encoded)
}

/// Encode environment variables as PowerShell assignment statements.
fn encode_env_vars_for_powershell(env: &HashMap<String, String>) -> Result<String, String> {
    let mut encoded = String::new();
    for (k, v) in env {
        if !is_valid_env_var_name(k) {
            return Err(format!("invalid env var name: {:?}", k));
        }
        encoded.push_str(&format!("$env:{} = {}\n", k, hard_quote_powershell(v)));
    }
    Ok(encoded)
}

/// Encode environment variables for the given shell type.
pub fn encode_env_vars_for_shell(
    shell_type: &str,
    env: &HashMap<String, String>,
) -> Result<String, String> {
    match shell_type {
        SHELL_TYPE_BASH | SHELL_TYPE_ZSH => encode_env_vars_for_bash(env),
        SHELL_TYPE_FISH => encode_env_vars_for_fish(env),
        SHELL_TYPE_PWSH => encode_env_vars_for_powershell(env),
        _ => Err(format!(
            "unknown or unsupported shell type for env var encoding: {}",
            shell_type
        )),
    }
}

// ---- Token swap ----

/// An unpacked swap token containing connection info.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnpackedToken {
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sockname: Option<String>,
}

impl UnpackedToken {
    /// Pack the token into a base64-encoded JSON string.
    pub fn pack(&self) -> Result<String, String> {
        let json = serde_json::to_string(self).map_err(|e| format!("json encode error: {}", e))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(json.as_bytes()))
    }

    /// Unpack a base64-encoded JSON token string.
    pub fn unpack(encoded: &str) -> Result<Self, String> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| format!("base64 decode error: {}", e))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("json decode error: {}", e))
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    // -- Quoting tests (from Go's shellquote_test.go) --

    #[test]
    fn test_hard_quote_simple() {
        assert_eq!(hard_quote("simple"), "simple");
    }

    #[test]
    fn test_hard_quote_safe_path() {
        assert_eq!(hard_quote("path/to/file.txt"), "path/to/file.txt");
    }

    #[test]
    fn test_hard_quote_empty() {
        assert_eq!(hard_quote(""), "\"\"");
    }

    #[test]
    fn test_hard_quote_tilde() {
        assert_eq!(hard_quote("~"), "\"~\"");
    }

    #[test]
    fn test_hard_quote_tilde_path() {
        assert_eq!(hard_quote("~/foo"), "\"~/foo\"");
    }

    #[test]
    fn test_hard_quote_tilde_spaces() {
        assert_eq!(hard_quote("~/foo bar"), "\"~/foo bar\"");
    }

    #[test]
    fn test_hard_quote_tilde_variable() {
        assert_eq!(hard_quote("~/foo$bar"), "\"~/foo\\$bar\"");
    }

    #[test]
    fn test_hard_quote_variable_start() {
        assert_eq!(hard_quote("$HOME/.config"), "\"\\$HOME/.config\"");
    }

    #[test]
    fn test_hard_quote_double_quotes() {
        assert_eq!(hard_quote("has \"quotes\""), "\"has \\\"quotes\\\"\"");
    }

    #[test]
    fn test_hard_quote_backslash() {
        assert_eq!(hard_quote("back\\slash"), "\"back\\\\slash\"");
    }

    #[test]
    fn test_hard_quote_backtick() {
        assert_eq!(hard_quote("`cmd`"), "\"\\`cmd\\`\"");
    }

    #[test]
    fn test_hard_quote_spaces() {
        assert_eq!(hard_quote("spaces here"), "\"spaces here\"");
    }

    #[test]
    fn test_soft_quote_simple() {
        assert_eq!(soft_quote("simple"), "simple");
    }

    #[test]
    fn test_soft_quote_safe_path() {
        assert_eq!(soft_quote("path/to/file.txt"), "path/to/file.txt");
    }

    #[test]
    fn test_soft_quote_empty() {
        assert_eq!(soft_quote(""), "\"\"");
    }

    #[test]
    fn test_soft_quote_tilde() {
        assert_eq!(soft_quote("~"), "~");
    }

    #[test]
    fn test_soft_quote_tilde_path() {
        assert_eq!(soft_quote("~/foo"), "~/foo");
    }

    #[test]
    fn test_soft_quote_tilde_spaces() {
        assert_eq!(soft_quote("~/foo bar"), "~\"/foo bar\"");
    }

    #[test]
    fn test_soft_quote_tilde_variable() {
        assert_eq!(soft_quote("~/foo$bar"), "~\"/foo$bar\"");
    }

    #[test]
    fn test_soft_quote_invalid_tilde() {
        assert_eq!(soft_quote("~foo"), "\"~foo\"");
    }

    #[test]
    fn test_soft_quote_variable_start() {
        assert_eq!(soft_quote("$HOME/.config"), "\"$HOME/.config\"");
    }

    #[test]
    fn test_soft_quote_variable_middle() {
        assert_eq!(soft_quote("prefix$HOME"), "\"prefix$HOME\"");
    }

    #[test]
    fn test_soft_quote_double_quotes() {
        assert_eq!(soft_quote("has \"quotes\""), "\"has \\\"quotes\\\"\"");
    }

    #[test]
    fn test_soft_quote_backslash() {
        assert_eq!(soft_quote("back\\slash"), "\"back\\\\slash\"");
    }

    #[test]
    fn test_soft_quote_backtick() {
        assert_eq!(soft_quote("`cmd`"), "\"\\`cmd\\`\"");
    }

    #[test]
    fn test_soft_quote_spaces() {
        assert_eq!(soft_quote("spaces here"), "\"spaces here\"");
    }

    // -- Fish quoting tests --

    #[test]
    fn test_hard_quote_fish_simple() {
        assert_eq!(hard_quote_fish("simple"), "simple");
    }

    #[test]
    fn test_hard_quote_fish_empty() {
        assert_eq!(hard_quote_fish(""), "\"\"");
    }

    #[test]
    fn test_hard_quote_fish_dollar() {
        assert_eq!(hard_quote_fish("$VAR"), "\"\\$VAR\"");
    }

    #[test]
    fn test_hard_quote_fish_backtick_not_escaped() {
        // Fish does NOT escape backticks
        assert_eq!(hard_quote_fish("`cmd`"), "\"`cmd`\"");
    }

    // -- PowerShell quoting tests --

    #[test]
    fn test_hard_quote_powershell_simple() {
        // Note: PowerShell always quotes (no safe pattern shortcut)
        assert_eq!(hard_quote_powershell("simple"), "\"simple\"");
    }

    #[test]
    fn test_hard_quote_powershell_empty() {
        assert_eq!(hard_quote_powershell(""), "\"\"");
    }

    #[test]
    fn test_hard_quote_powershell_dollar() {
        assert_eq!(hard_quote_powershell("$env:PATH"), "\"`$env:PATH\"");
    }

    #[test]
    fn test_hard_quote_powershell_backtick() {
        assert_eq!(hard_quote_powershell("`cmd`"), "\"``cmd``\"");
    }

    // -- Env var name validation --

    #[test]
    fn test_is_valid_env_var_name() {
        assert!(is_valid_env_var_name("HOME"));
        assert!(is_valid_env_var_name("_PRIVATE"));
        assert!(is_valid_env_var_name("MY_VAR_123"));
        assert!(!is_valid_env_var_name(""));
        assert!(!is_valid_env_var_name("123ABC"));
        assert!(!is_valid_env_var_name("MY-VAR"));
        assert!(!is_valid_env_var_name("MY VAR"));
    }

    // -- Shell detection --

    #[test]
    fn test_get_shell_type_from_path() {
        assert_eq!(get_shell_type_from_path("/bin/bash"), SHELL_TYPE_BASH);
        assert_eq!(get_shell_type_from_path("/usr/bin/zsh"), SHELL_TYPE_ZSH);
        assert_eq!(get_shell_type_from_path("/usr/bin/fish"), SHELL_TYPE_FISH);
        assert_eq!(get_shell_type_from_path("/usr/bin/pwsh"), SHELL_TYPE_PWSH);
        assert_eq!(
            get_shell_type_from_path("C:\\Windows\\powershell.exe"),
            SHELL_TYPE_PWSH
        );
        assert_eq!(get_shell_type_from_path("/bin/sh"), SHELL_TYPE_UNKNOWN);
    }

    #[test]
    fn test_detect_local_shell_path() {
        let path = detect_local_shell_path();
        assert!(!path.is_empty());
    }

    // -- Env str key --

    #[test]
    fn test_get_env_str_key() {
        assert_eq!(get_env_str_key("HOME=/home/user"), "HOME");
        assert_eq!(get_env_str_key("PATH=/usr/bin"), "PATH");
        assert_eq!(get_env_str_key("NOVALUE"), "NOVALUE");
    }

    // -- OSC formatting --

    #[test]
    fn test_format_osc() {
        assert_eq!(format_osc(7, &[]), "\x1b]7\x07");
        assert_eq!(format_osc(133, &["A"]), "\x1b]133;A\x07");
        assert_eq!(format_osc(1337, &["a", "b"]), "\x1b]1337;a;b\x07");
    }

    // -- Environment encoding --

    #[test]
    fn test_encode_env_vars_bash() {
        let mut env = HashMap::new();
        env.insert("HOME".to_string(), "/home/user".to_string());
        let result = encode_env_vars_for_bash(&env).unwrap();
        assert!(result.contains("export HOME=/home/user\n"));
    }

    #[test]
    fn test_encode_env_vars_bash_with_spaces() {
        let mut env = HashMap::new();
        env.insert("MSG".to_string(), "hello world".to_string());
        let result = encode_env_vars_for_bash(&env).unwrap();
        assert!(result.contains("export MSG=\"hello world\"\n"));
    }

    #[test]
    fn test_encode_env_vars_fish() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        let result = encode_env_vars_for_fish(&env).unwrap();
        assert!(result.contains("set -x PATH /usr/bin\n"));
    }

    #[test]
    fn test_encode_env_vars_powershell() {
        let mut env = HashMap::new();
        env.insert("VAR".to_string(), "value".to_string());
        let result = encode_env_vars_for_powershell(&env).unwrap();
        assert!(result.contains("$env:VAR = \"value\"\n"));
    }

    #[test]
    fn test_encode_env_vars_invalid_name() {
        let mut env = HashMap::new();
        env.insert("INVALID-NAME".to_string(), "value".to_string());
        assert!(encode_env_vars_for_bash(&env).is_err());
        assert!(encode_env_vars_for_fish(&env).is_err());
        assert!(encode_env_vars_for_powershell(&env).is_err());
    }

    #[test]
    fn test_encode_env_vars_for_shell_dispatch() {
        let mut env = HashMap::new();
        env.insert("X".to_string(), "1".to_string());
        assert!(encode_env_vars_for_shell("bash", &env).is_ok());
        assert!(encode_env_vars_for_shell("zsh", &env).is_ok());
        assert!(encode_env_vars_for_shell("fish", &env).is_ok());
        assert!(encode_env_vars_for_shell("pwsh", &env).is_ok());
        assert!(encode_env_vars_for_shell("unknown", &env).is_err());
    }

    // -- Token swap --

    #[test]
    fn test_unpacked_token_pack_unpack() {
        let token = UnpackedToken {
            token: "test-uuid-123".to_string(),
            sockname: Some("mysock".to_string()),
        };
        let packed = token.pack().unwrap();
        let unpacked = UnpackedToken::unpack(&packed).unwrap();
        assert_eq!(unpacked.token, "test-uuid-123");
        assert_eq!(unpacked.sockname.as_deref(), Some("mysock"));
    }

    #[test]
    fn test_unpacked_token_no_sockname() {
        let token = UnpackedToken {
            token: "abc".to_string(),
            sockname: None,
        };
        let packed = token.pack().unwrap();
        let unpacked = UnpackedToken::unpack(&packed).unwrap();
        assert_eq!(unpacked.token, "abc");
        assert!(unpacked.sockname.is_none());
    }

    #[test]
    fn test_unpack_invalid_base64() {
        assert!(UnpackedToken::unpack("not-valid-base64!!!").is_err());
    }

    // -- Constants --

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_TERM_TYPE, "xterm-256color");
        assert_eq!(DEFAULT_TERM_ROWS, 24);
        assert_eq!(DEFAULT_TERM_COLS, 80);
        assert_eq!(DEFAULT_SHELL_PATH, "/bin/bash");
        assert_eq!(MAX_QUOTE_SIZE, 10_000_000);
    }
}
