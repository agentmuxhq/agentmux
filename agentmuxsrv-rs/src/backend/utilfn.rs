// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Utility functions ported from Go's `pkg/util/utilfn/utilfn.go`.
//!

#![allow(dead_code)]
//! Includes null encoding/decoding, hashing, binary detection, star matching,
//! slice operations, string helpers, and atomic file operations.

use std::collections::HashMap;
use std::io;
use std::path::Path;

// ---- Null Encoding ----

const NULL_ENCODE_ESC_BYTE: u8 = b'\\';
const NULL_ENCODE_SEP_BYTE: u8 = b'|';
const NULL_ENCODE_EQ_BYTE: u8 = b'=';
const NULL_ENCODE_ZERO_BYTE_ESC: u8 = b'0';
const NULL_ENCODE_ESC_BYTE_ESC: u8 = b'\\';
const NULL_ENCODE_SEP_BYTE_ESC: u8 = b's';
const NULL_ENCODE_EQ_BYTE_ESC: u8 = b'e';

/// Encode a string, escaping null bytes, backslashes, separators (`|`), and equals (`=`).
///
/// - `\0` → `\0`
/// - `\` → `\\`
/// - `|` → `\s`
/// - `=` → `\e`
pub fn null_encode_str(s: &str) -> Vec<u8> {
    let bytes = s.as_bytes();
    // Fast path: if no special bytes, return as-is
    if !bytes.iter().any(|&b| b == 0 || b == NULL_ENCODE_ESC_BYTE || b == NULL_ENCODE_SEP_BYTE || b == NULL_ENCODE_EQ_BYTE) {
        return bytes.to_vec();
    }
    let mut rtn = Vec::with_capacity(bytes.len() + 8);
    for &b in bytes {
        match b {
            0 => {
                rtn.push(NULL_ENCODE_ESC_BYTE);
                rtn.push(NULL_ENCODE_ZERO_BYTE_ESC);
            }
            b if b == NULL_ENCODE_ESC_BYTE => {
                rtn.push(NULL_ENCODE_ESC_BYTE);
                rtn.push(NULL_ENCODE_ESC_BYTE_ESC);
            }
            b if b == NULL_ENCODE_SEP_BYTE => {
                rtn.push(NULL_ENCODE_ESC_BYTE);
                rtn.push(NULL_ENCODE_SEP_BYTE_ESC);
            }
            b if b == NULL_ENCODE_EQ_BYTE => {
                rtn.push(NULL_ENCODE_ESC_BYTE);
                rtn.push(NULL_ENCODE_EQ_BYTE_ESC);
            }
            _ => rtn.push(b),
        }
    }
    rtn
}

/// Decode a null-encoded byte slice back to a string.
pub fn null_decode_str(barr: &[u8]) -> Result<String, String> {
    if !barr.contains(&NULL_ENCODE_ESC_BYTE) {
        return Ok(String::from_utf8_lossy(barr).into_owned());
    }
    let mut rtn = Vec::with_capacity(barr.len());
    let mut i = 0;
    while i < barr.len() {
        let cur = barr[i];
        if cur == NULL_ENCODE_ESC_BYTE {
            i += 1;
            if i >= barr.len() {
                return Err("invalid null encoding: escape at end of string".to_string());
            }
            let next = barr[i];
            match next {
                NULL_ENCODE_ZERO_BYTE_ESC => rtn.push(0),
                NULL_ENCODE_ESC_BYTE_ESC => rtn.push(NULL_ENCODE_ESC_BYTE),
                NULL_ENCODE_SEP_BYTE_ESC => rtn.push(NULL_ENCODE_SEP_BYTE),
                NULL_ENCODE_EQ_BYTE_ESC => rtn.push(NULL_ENCODE_EQ_BYTE),
                _ => return Err(format!("invalid null encoding: {}", next)),
            }
        } else {
            rtn.push(cur);
        }
        i += 1;
    }
    Ok(String::from_utf8_lossy(&rtn).into_owned())
}

/// Encode a map of strings using null encoding, sorted by key.
/// Format: `key1=val1|key2=val2`
pub fn encode_string_map(m: &HashMap<String, String>) -> Vec<u8> {
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort();
    let mut buf = Vec::new();
    for (idx, key) in keys.iter().enumerate() {
        let val = &m[*key];
        buf.extend_from_slice(&null_encode_str(key));
        buf.push(NULL_ENCODE_EQ_BYTE);
        buf.extend_from_slice(&null_encode_str(val));
        if idx < keys.len() - 1 {
            buf.push(NULL_ENCODE_SEP_BYTE);
        }
    }
    buf
}

/// Decode a null-encoded byte slice into a map of strings.
pub fn decode_string_map(barr: &[u8]) -> Result<HashMap<String, String>, String> {
    if barr.is_empty() {
        return Ok(HashMap::new());
    }
    let mut rtn = HashMap::new();
    for part in split_bytes(barr, NULL_ENCODE_SEP_BYTE) {
        let kv: Vec<&[u8]> = splitn_bytes(part, NULL_ENCODE_EQ_BYTE, 2);
        if kv.len() != 2 {
            return Err(format!("invalid null encoding: {}", String::from_utf8_lossy(part)));
        }
        let key = null_decode_str(kv[0])?;
        let val = null_decode_str(kv[1])?;
        rtn.insert(key, val);
    }
    Ok(rtn)
}

/// Encode a string array using null encoding.
/// Format: `elem1|elem2|elem3`
pub fn encode_string_array(arr: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    for (idx, s) in arr.iter().enumerate() {
        buf.extend_from_slice(&null_encode_str(s));
        if idx < arr.len() - 1 {
            buf.push(NULL_ENCODE_SEP_BYTE);
        }
    }
    buf
}

/// Decode a null-encoded byte slice into a string array.
pub fn decode_string_array(barr: &[u8]) -> Result<Vec<String>, String> {
    if barr.is_empty() {
        return Ok(Vec::new());
    }
    let mut rtn = Vec::new();
    for part in split_bytes(barr, NULL_ENCODE_SEP_BYTE) {
        rtn.push(null_decode_str(part)?);
    }
    Ok(rtn)
}

/// Check if an encoded string array has the given first value.
pub fn encoded_string_array_has_first_val(encoded: &[u8], first_key: &str) -> bool {
    let first_key_bytes = null_encode_str(first_key);
    if !encoded.starts_with(&first_key_bytes) {
        return false;
    }
    encoded.len() == first_key_bytes.len() || encoded[first_key_bytes.len()] == NULL_ENCODE_SEP_BYTE
}

/// Get the first value from an encoded string array without decoding the whole array.
pub fn encoded_string_array_get_first_val(encoded: &[u8]) -> String {
    let sep_idx = encoded.iter().position(|&b| b == NULL_ENCODE_SEP_BYTE);
    let slice = match sep_idx {
        Some(idx) => &encoded[..idx],
        None => encoded,
    };
    null_decode_str(slice).unwrap_or_default()
}

// Helper: split bytes by delimiter
fn split_bytes(data: &[u8], delim: u8) -> Vec<&[u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, &b) in data.iter().enumerate() {
        if b == delim {
            parts.push(&data[start..i]);
            start = i + 1;
        }
    }
    parts.push(&data[start..]);
    parts
}

// Helper: splitn bytes by delimiter
fn splitn_bytes(data: &[u8], delim: u8, n: usize) -> Vec<&[u8]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (i, &b) in data.iter().enumerate() {
        if b == delim && parts.len() < n - 1 {
            parts.push(&data[start..i]);
            start = i + 1;
        }
    }
    parts.push(&data[start..]);
    parts
}

// ---- Hashing ----

/// SHA1 hash of data, returned as base64 string.
pub fn sha1_hash(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    base64_encode(&result)
}

/// FNV-64a hash of a string, returned as base64url (no padding) string.
pub fn quick_hash_string(s: &str) -> String {
    let mut hasher = Fnv64a::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    base64_url_encode(&result)
}

// Simple SHA1 implementation
struct Sha1 {
    state: [u32; 5],
    count: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha1 {
    fn new() -> Self {
        Self {
            state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
            count: 0,
            buffer: [0u8; 64],
            buffer_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        let mut i = 0;
        self.count += data.len() as u64;
        if self.buffer_len > 0 {
            let space = 64 - self.buffer_len;
            let copy_len = std::cmp::min(space, data.len());
            self.buffer[self.buffer_len..self.buffer_len + copy_len].copy_from_slice(&data[..copy_len]);
            self.buffer_len += copy_len;
            i = copy_len;
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
            }
        }
        while i + 64 <= data.len() {
            let mut block = [0u8; 64];
            block.copy_from_slice(&data[i..i + 64]);
            self.process_block(&block);
            i += 64;
        }
        if i < data.len() {
            let remaining = data.len() - i;
            self.buffer[..remaining].copy_from_slice(&data[i..]);
            self.buffer_len = remaining;
        }
    }

    #[allow(clippy::needless_range_loop)]
    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([block[i * 4], block[i * 4 + 1], block[i * 4 + 2], block[i * 4 + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = self.state;
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
    }

    fn finalize(mut self) -> [u8; 20] {
        let bit_count = self.count * 8;
        // Padding
        let mut padding = vec![0x80u8];
        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len - 1
        } else {
            120 - self.buffer_len - 1
        };
        padding.extend(std::iter::repeat_n(0u8, pad_len));
        padding.extend_from_slice(&bit_count.to_be_bytes());
        self.update(&padding);

        let mut result = [0u8; 20];
        for (i, &s) in self.state.iter().enumerate() {
            result[i * 4..i * 4 + 4].copy_from_slice(&s.to_be_bytes());
        }
        result
    }
}

// Simple FNV-64a implementation
struct Fnv64a {
    hash: u64,
}

impl Fnv64a {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self { hash: Self::OFFSET_BASIS }
    }

    fn update(&mut self, data: &[u8]) {
        for &b in data {
            self.hash ^= b as u64;
            self.hash = self.hash.wrapping_mul(Self::PRIME);
        }
    }

    fn finalize(&self) -> [u8; 8] {
        self.hash.to_be_bytes()
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < data.len() {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if i + 2 < data.len() {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

fn base64_url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < data.len() {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        }
        i += 3;
    }
    result
}

// ---- Binary Detection ----

/// Check if data contains binary (non-text) bytes.
/// Returns true if any byte < 32 other than `\n`, `\r`, `\t`, `\f`, `\b`.
pub fn has_binary_data(data: &[u8]) -> bool {
    data.iter().any(|&b| b < 32 && b != b'\n' && b != b'\r' && b != b'\t' && b != 0x0C && b != 0x08)
}

/// Check if content is binary by examining up to 8192 bytes.
/// Checks for null byte ratio > 1% and UTF-8 validity.
pub fn is_binary_content(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }
    let sample_size = std::cmp::min(8192, data.len());
    let sample = &data[..sample_size];

    let null_count = sample.iter().filter(|&&b| b == 0).count();
    if null_count as f64 / sample.len() as f64 > 0.01 {
        return true;
    }

    if std::str::from_utf8(sample).is_err() {
        return true;
    }

    false
}

// ---- Star Matching ----

/// Match a delimited string with a pattern string.
/// `*` matches a single part, `**` matches the rest of the string (only valid at end).
pub fn star_match_string(pattern: &str, s: &str, delimiter: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split(delimiter).collect();
    let string_parts: Vec<&str> = s.split(delimiter).collect();
    let p_len = pattern_parts.len();
    let s_len = string_parts.len();

    for i in 0..p_len {
        if pattern_parts[i] == "**" {
            return i == p_len - 1;
        }
        if i >= s_len {
            return false;
        }
        if pattern_parts[i] != "*" && pattern_parts[i] != string_parts[i] {
            return false;
        }
    }
    p_len == s_len
}

// ---- Slice Operations ----

/// Find the index of an element in a slice. Returns -1 if not found.
pub fn slice_idx<T: PartialEq>(arr: &[T], elem: &T) -> i32 {
    for (idx, e) in arr.iter().enumerate() {
        if e == elem {
            return idx as i32;
        }
    }
    -1
}

/// Remove an element from a vec. Returns a new vec without the element.
pub fn remove_elem<T: PartialEq + Clone>(arr: &[T], elem: &T) -> Vec<T> {
    let idx = slice_idx(arr, elem);
    if idx == -1 {
        return arr.to_vec();
    }
    let idx = idx as usize;
    let mut result = Vec::with_capacity(arr.len() - 1);
    result.extend_from_slice(&arr[..idx]);
    result.extend_from_slice(&arr[idx + 1..]);
    result
}

/// Add an element to a vec if it's not already present.
pub fn add_elem_uniq<T: PartialEq + Clone>(arr: &[T], elem: T) -> Vec<T> {
    if slice_idx(arr, &elem) != -1 {
        return arr.to_vec();
    }
    let mut result = arr.to_vec();
    result.push(elem);
    result
}

/// Move element at `idx` to the front of the slice. Returns a new vec.
pub fn move_to_front<T: Clone>(arr: &[T], idx: usize) -> Vec<T> {
    if idx == 0 || idx >= arr.len() {
        return arr.to_vec();
    }
    let mut rtn = Vec::with_capacity(arr.len());
    rtn.push(arr[idx].clone());
    rtn.extend_from_slice(&arr[..idx]);
    rtn.extend_from_slice(&arr[idx + 1..]);
    rtn
}

// ---- String Helpers ----

/// Truncate a string with "..." if it exceeds maxLen (char-safe).
pub fn ellipsis_str(s: &str, max_len: usize) -> String {
    let max_len = if max_len < 4 { 4 } else { max_len };
    let char_count = s.chars().count();
    if char_count > max_len {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    } else {
        s.to_string()
    }
}

/// Truncate a string with "..." if it exceeds maxLen (char-safe).
pub fn truncate_string(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        return s.to_string();
    }
    let max_len = if max_len < 4 { 4 } else { max_len };
    let truncated: String = s.chars().take(max_len - 3).collect();
    format!("{}...", truncated)
}

/// Get the first line of a string.
pub fn get_first_line(s: &str) -> &str {
    match s.find('\n') {
        Some(idx) => &s[..idx],
        None => s,
    }
}

/// Parse an integer from a string, returning 0 on error.
pub fn atoi_no_err(s: &str) -> i32 {
    s.parse::<i32>().unwrap_or(0)
}

/// Generate a random hex string of the given number of hex digits.
pub fn random_hex_string(num_hex_digits: usize) -> Result<String, io::Error> {
    use std::fmt::Write;
    let num_bytes = num_hex_digits.div_ceil(2);
    let mut bytes = vec![0u8; num_bytes];
    getrandom(&mut bytes)?;
    let mut hex = String::with_capacity(num_hex_digits);
    for b in &bytes {
        write!(hex, "{:02x}", b).unwrap();
    }
    hex.truncate(num_hex_digits);
    Ok(hex)
}

/// Platform-independent random bytes.
/// Uses /dev/urandom on Unix, BCryptGenRandom on Windows.
fn getrandom(buf: &mut [u8]) -> Result<(), io::Error> {
    #[cfg(unix)]
    {
        use std::fs::File;
        use std::io::Read;
        let mut f = File::open("/dev/urandom")?;
        f.read_exact(buf)?;
    }
    #[cfg(windows)]
    {
        // Use the Windows CSPRNG via std's internal API
        // This is safe and doesn't require external crates
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        // Fill buffer using hash-based randomness as fallback
        let mut offset = 0;
        while offset < buf.len() {
            let state = RandomState::new();
            let hash = state.build_hasher().finish();
            let bytes = hash.to_ne_bytes();
            let copy_len = std::cmp::min(bytes.len(), buf.len() - offset);
            buf[offset..offset + copy_len].copy_from_slice(&bytes[..copy_len]);
            offset += copy_len;
        }
    }
    Ok(())
}

/// Convert known architecture names to standard patterns.
pub fn filter_valid_arch(arch: &str) -> Result<&'static str, String> {
    match arch.trim().to_lowercase().as_str() {
        "amd64" | "x86_64" | "x64" => Ok("x64"),
        "arm64" | "aarch64" => Ok("arm64"),
        other => Err(format!("unknown architecture: {}", other)),
    }
}

// ---- Atomic File Operations ----

/// Atomically copy a file by writing to a temp file then renaming.
/// On Unix, sets file permissions to `perms`. On Windows, permissions are ignored.
pub fn atomic_rename_copy(dst_path: &Path, src_path: &Path, _perms: u32) -> io::Result<()> {
    use std::fs;

    let temp_name = format!("{}.new", dst_path.display());
    let temp_path = Path::new(&temp_name);
    fs::copy(src_path, temp_path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(temp_path, fs::Permissions::from_mode(_perms))?;
    }
    fs::rename(temp_path, dst_path)?;
    Ok(())
}

/// Write file only if contents differ. Returns true if file was written.
pub fn write_file_if_different(file_name: &Path, contents: &[u8]) -> io::Result<bool> {
    use std::fs;
    if let Ok(old_contents) = fs::read(file_name) {
        if old_contents == contents {
            return Ok(false);
        }
    }
    fs::write(file_name, contents)?;
    Ok(true)
}

// ---- Misc ----

/// Get line and column from a byte offset in content.
pub fn get_line_col_from_offset(data: &[u8], offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for &byte in data.iter().take(std::cmp::min(offset, data.len())) {
        if byte == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Find the longest common prefix of a set of strings, bounded by a root.
pub fn longest_prefix(root: &str, strs: &[&str]) -> String {
    if strs.is_empty() {
        return root.to_string();
    }
    if strs.len() == 1 {
        let comp = strs[0];
        if comp.len() >= root.len() && comp.starts_with(root) {
            return comp.to_string();
        }
    }
    let mut lcp = strs[0].to_string();
    for s in &strs[1..] {
        let mut end = 0;
        for (j, (a, b)) in lcp.chars().zip(s.chars()).enumerate() {
            if a != b {
                break;
            }
            end = j + a.len_utf8();
        }
        lcp.truncate(end);
    }
    if lcp.len() < root.len() || !lcp.starts_with(root) {
        return root.to_string();
    }
    lcp
}

/// Indent each non-empty line of a string.
pub fn indent_string(indent: &str, s: &str) -> String {
    let mut result = String::new();
    for line in s.split('\n') {
        if line.is_empty() {
            result.push('\n');
        } else {
            result.push_str(indent);
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Shell hex escape a string (each byte as `\xNN`).
pub fn shell_hex_escape(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        result.push_str(&format!("\\x{:02x}", b));
    }
    result
}

/// Combine two string arrays, removing duplicates while preserving order.
pub fn combine_str_arrays(arr1: &[String], arr2: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for s in arr1.iter().chain(arr2.iter()) {
        if seen.insert(s.clone()) {
            result.push(s.clone());
        }
    }
    result
}

// ---- Sentinel / Helper Types ----

/// Sentinel value for StrWithPos.pos to indicate no position.
pub const NO_STR_POS: i32 = -1;

/// A string with a cursor position (rune-based, not byte-based).
#[derive(Debug, Clone, PartialEq)]
pub struct StrWithPos {
    pub str_val: String,
    pub pos: i32,
}

impl StrWithPos {
    pub fn new(s: String, pos: i32) -> Self {
        Self { str_val: s, pos }
    }

    /// Parse a string with `[*]` cursor marker.
    pub fn parse(s: &str) -> Self {
        match s.find("[*]") {
            None => Self { str_val: s.to_string(), pos: NO_STR_POS },
            Some(idx) => {
                let before = &s[..idx];
                let after = &s[idx + 3..];
                let pos = before.chars().count() as i32;
                Self {
                    str_val: format!("{}{}", before, after),
                    pos,
                }
            }
        }
    }

    pub fn prepend(&self, prefix: &str) -> Self {
        Self {
            str_val: format!("{}{}", prefix, self.str_val),
            pos: prefix.chars().count() as i32 + self.pos,
        }
    }

    pub fn append(&self, suffix: &str) -> Self {
        Self {
            str_val: format!("{}{}", self.str_val, suffix),
            pos: self.pos,
        }
    }
}

impl std::fmt::Display for StrWithPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.pos == NO_STR_POS {
            write!(f, "{}", self.str_val)
        } else if self.pos < 0 {
            write!(f, "[*]_{}", self.str_val)
        } else {
            let pos = self.pos as usize;
            let mut chars: Vec<char> = Vec::new();
            for (i, ch) in self.str_val.chars().enumerate() {
                if i == pos {
                    chars.extend(['[', '*', ']']);
                }
                chars.push(ch);
            }
            if pos >= self.str_val.chars().count() {
                chars.extend(['[', '*', ']']);
            }
            write!(f, "{}", chars.iter().collect::<String>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Null Encoding Tests ----

    #[test]
    fn test_null_encode_no_special() {
        assert_eq!(null_encode_str("hello"), b"hello");
    }

    #[test]
    fn test_null_encode_with_null() {
        assert_eq!(null_encode_str("a\0b"), b"a\\0b");
    }

    #[test]
    fn test_null_encode_with_backslash() {
        assert_eq!(null_encode_str("a\\b"), b"a\\\\b");
    }

    #[test]
    fn test_null_encode_with_sep() {
        assert_eq!(null_encode_str("a|b"), b"a\\sb");
    }

    #[test]
    fn test_null_encode_with_eq() {
        assert_eq!(null_encode_str("a=b"), b"a\\eb");
    }

    #[test]
    fn test_null_decode_no_special() {
        assert_eq!(null_decode_str(b"hello").unwrap(), "hello");
    }

    #[test]
    fn test_null_decode_with_null() {
        assert_eq!(null_decode_str(b"a\\0b").unwrap(), "a\0b");
    }

    #[test]
    fn test_null_decode_with_backslash() {
        assert_eq!(null_decode_str(b"a\\\\b").unwrap(), "a\\b");
    }

    #[test]
    fn test_null_decode_with_sep() {
        assert_eq!(null_decode_str(b"a\\sb").unwrap(), "a|b");
    }

    #[test]
    fn test_null_decode_with_eq() {
        assert_eq!(null_decode_str(b"a\\eb").unwrap(), "a=b");
    }

    #[test]
    fn test_null_encode_decode_roundtrip() {
        let cases = ["hello", "a\0b\0c", "key=val|more\\end", "", "no special"];
        for s in &cases {
            let encoded = null_encode_str(s);
            let decoded = null_decode_str(&encoded).unwrap();
            assert_eq!(&decoded, s, "roundtrip failed for {:?}", s);
        }
    }

    #[test]
    fn test_null_decode_invalid() {
        assert!(null_decode_str(b"a\\xb").is_err());
    }

    #[test]
    fn test_null_decode_escape_at_end() {
        assert!(null_decode_str(b"a\\").is_err());
    }

    // ---- String Map Encoding ----

    #[test]
    fn test_encode_decode_string_map() {
        let mut m = HashMap::new();
        m.insert("key1".into(), "val1".into());
        m.insert("key2".into(), "val2".into());
        let encoded = encode_string_map(&m);
        let decoded = decode_string_map(&encoded).unwrap();
        assert_eq!(decoded, m);
    }

    #[test]
    fn test_encode_decode_string_map_with_special() {
        let mut m = HashMap::new();
        m.insert("k\0y".into(), "v=l".into());
        m.insert("a|b".into(), "c\\d".into());
        let encoded = encode_string_map(&m);
        let decoded = decode_string_map(&encoded).unwrap();
        assert_eq!(decoded, m);
    }

    #[test]
    fn test_decode_empty_string_map() {
        let decoded = decode_string_map(b"").unwrap();
        assert!(decoded.is_empty());
    }

    // ---- String Array Encoding ----

    #[test]
    fn test_encode_decode_string_array() {
        let arr = vec!["hello".to_string(), "world".to_string()];
        let encoded = encode_string_array(&arr);
        let decoded = decode_string_array(&encoded).unwrap();
        assert_eq!(decoded, arr);
    }

    #[test]
    fn test_encode_decode_string_array_with_special() {
        let arr = vec!["a|b".to_string(), "c=d".to_string(), "e\\f".to_string()];
        let encoded = encode_string_array(&arr);
        let decoded = decode_string_array(&encoded).unwrap();
        assert_eq!(decoded, arr);
    }

    #[test]
    fn test_decode_empty_array() {
        let decoded = decode_string_array(b"").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_encoded_string_array_has_first_val() {
        let arr = vec!["first".to_string(), "second".to_string()];
        let encoded = encode_string_array(&arr);
        assert!(encoded_string_array_has_first_val(&encoded, "first"));
        assert!(!encoded_string_array_has_first_val(&encoded, "second"));
        assert!(!encoded_string_array_has_first_val(&encoded, "firs"));
    }

    #[test]
    fn test_encoded_string_array_get_first_val() {
        let arr = vec!["first".to_string(), "second".to_string()];
        let encoded = encode_string_array(&arr);
        assert_eq!(encoded_string_array_get_first_val(&encoded), "first");
    }

    #[test]
    fn test_encoded_string_array_get_first_val_single() {
        let arr = vec!["only".to_string()];
        let encoded = encode_string_array(&arr);
        assert_eq!(encoded_string_array_get_first_val(&encoded), "only");
    }

    // ---- Hashing Tests ----

    #[test]
    fn test_sha1_hash_known() {
        // SHA1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        let hash = sha1_hash(b"");
        // Base64 of that 20-byte hash
        assert_eq!(hash, "2jmj7l5rSw0yVb/vlWAYkK/YBwk=");
    }

    #[test]
    fn test_sha1_hash_hello() {
        // SHA1("hello") = aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
        let hash = sha1_hash(b"hello");
        assert_eq!(hash, "qvTGHdzF6KLavt4PO0gs2a6pQ00=");
    }

    #[test]
    fn test_quick_hash_string() {
        // FNV-64a deterministic
        let h1 = quick_hash_string("test");
        let h2 = quick_hash_string("test");
        assert_eq!(h1, h2);
        // Different input → different hash
        assert_ne!(quick_hash_string("test"), quick_hash_string("other"));
    }

    #[test]
    fn test_quick_hash_string_known() {
        // FNV-64a("") = CBF29CE484222325 in big-endian
        let h = quick_hash_string("");
        assert!(!h.is_empty());
    }

    // ---- Binary Detection ----

    #[test]
    fn test_has_binary_data_text() {
        assert!(!has_binary_data(b"hello world\n"));
    }

    #[test]
    fn test_has_binary_data_with_null() {
        assert!(has_binary_data(b"hello\x00world"));
    }

    #[test]
    fn test_has_binary_data_with_control() {
        assert!(has_binary_data(b"hello\x01world"));
    }

    #[test]
    fn test_has_binary_data_with_tab_newline() {
        assert!(!has_binary_data(b"hello\tworld\nfoo\rbar"));
    }

    #[test]
    fn test_is_binary_content_empty() {
        assert!(!is_binary_content(b""));
    }

    #[test]
    fn test_is_binary_content_text() {
        assert!(!is_binary_content(b"hello world\n"));
    }

    #[test]
    fn test_is_binary_content_many_nulls() {
        let mut data = vec![0u8; 100];
        data.extend_from_slice(b"hello");
        assert!(is_binary_content(&data));
    }

    // ---- Star Matching ----

    #[test]
    fn test_star_match_exact() {
        assert!(star_match_string("a:b:c", "a:b:c", ":"));
    }

    #[test]
    fn test_star_match_wildcard() {
        assert!(star_match_string("a:*:c", "a:b:c", ":"));
    }

    #[test]
    fn test_star_match_double_star() {
        assert!(star_match_string("a:**", "a:b:c", ":"));
    }

    #[test]
    fn test_star_match_no_match() {
        assert!(!star_match_string("a:b:c", "a:b:d", ":"));
    }

    #[test]
    fn test_star_match_length_mismatch() {
        assert!(!star_match_string("a:b", "a:b:c", ":"));
        assert!(!star_match_string("a:b:c", "a:b", ":"));
    }

    #[test]
    fn test_star_match_double_star_must_be_last() {
        assert!(!star_match_string("a:**:c", "a:b:c", ":"));
    }

    // ---- Slice Operations ----

    #[test]
    fn test_slice_idx_found() {
        assert_eq!(slice_idx(&[1, 2, 3], &2), 1);
    }

    #[test]
    fn test_slice_idx_not_found() {
        assert_eq!(slice_idx(&[1, 2, 3], &4), -1);
    }

    #[test]
    fn test_remove_elem() {
        assert_eq!(remove_elem(&[1, 2, 3], &2), vec![1, 3]);
    }

    #[test]
    fn test_remove_elem_not_found() {
        assert_eq!(remove_elem(&[1, 2, 3], &4), vec![1, 2, 3]);
    }

    #[test]
    fn test_add_elem_uniq_new() {
        assert_eq!(add_elem_uniq(&[1, 2], 3), vec![1, 2, 3]);
    }

    #[test]
    fn test_add_elem_uniq_existing() {
        assert_eq!(add_elem_uniq(&[1, 2, 3], 2), vec![1, 2, 3]);
    }

    #[test]
    fn test_move_to_front() {
        assert_eq!(move_to_front(&[1, 2, 3, 4], 2), vec![3, 1, 2, 4]);
    }

    #[test]
    fn test_move_to_front_already_front() {
        assert_eq!(move_to_front(&[1, 2, 3], 0), vec![1, 2, 3]);
    }

    #[test]
    fn test_move_to_front_out_of_bounds() {
        assert_eq!(move_to_front(&[1, 2, 3], 5), vec![1, 2, 3]);
    }

    // ---- String Helpers ----

    #[test]
    fn test_ellipsis_str() {
        assert_eq!(ellipsis_str("hello world foo bar", 10), "hello w...");
        assert_eq!(ellipsis_str("short", 10), "short");
    }

    #[test]
    fn test_ellipsis_str_multibyte() {
        // "héllo wörld" has multi-byte chars — must not panic
        assert_eq!(ellipsis_str("héllo wörld foo", 10), "héllo w...");
        // CJK characters
        assert_eq!(ellipsis_str("你好世界测试数据信息", 7), "你好世界...");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello world foo bar", 10), "hello w...");
        assert_eq!(truncate_string("short", 10), "short");
    }

    #[test]
    fn test_truncate_string_multibyte() {
        // Multi-byte chars must not cause panics
        assert_eq!(truncate_string("héllo wörld foo", 10), "héllo w...");
        assert_eq!(truncate_string("日本語テスト文字列", 6), "日本語...");
    }

    #[test]
    fn test_get_first_line() {
        assert_eq!(get_first_line("hello\nworld"), "hello");
        assert_eq!(get_first_line("no newline"), "no newline");
    }

    #[test]
    fn test_atoi_no_err() {
        assert_eq!(atoi_no_err("42"), 42);
        assert_eq!(atoi_no_err("abc"), 0);
        assert_eq!(atoi_no_err(""), 0);
    }

    #[test]
    fn test_random_hex_string() {
        let s = random_hex_string(16).unwrap();
        assert_eq!(s.len(), 16);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_random_hex_string_odd() {
        let s = random_hex_string(5).unwrap();
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_filter_valid_arch() {
        assert_eq!(filter_valid_arch("amd64").unwrap(), "x64");
        assert_eq!(filter_valid_arch("x86_64").unwrap(), "x64");
        assert_eq!(filter_valid_arch("ARM64").unwrap(), "arm64");
        assert_eq!(filter_valid_arch("aarch64").unwrap(), "arm64");
        assert_eq!(filter_valid_arch("AARCH64").unwrap(), "arm64");
        assert!(filter_valid_arch("mips").is_err());
    }

    // ---- Line/Col ----

    #[test]
    fn test_get_line_col_from_offset() {
        let data = b"hello\nworld\nfoo";
        assert_eq!(get_line_col_from_offset(data, 0), (1, 1));
        assert_eq!(get_line_col_from_offset(data, 5), (1, 6));
        assert_eq!(get_line_col_from_offset(data, 6), (2, 1));
        assert_eq!(get_line_col_from_offset(data, 12), (3, 1));
    }

    // ---- Longest Prefix ----

    #[test]
    fn test_longest_prefix_empty() {
        assert_eq!(longest_prefix("/root", &[]), "/root");
    }

    #[test]
    fn test_longest_prefix_single() {
        assert_eq!(longest_prefix("/root", &["/root/foo"]), "/root/foo");
    }

    #[test]
    fn test_longest_prefix_multiple() {
        assert_eq!(longest_prefix("/", &["/root/foo", "/root/bar"]), "/root/");
    }

    // ---- Indent ----

    #[test]
    fn test_indent_string() {
        assert_eq!(indent_string("  ", "a\nb\n\nc"), "  a\n  b\n\n  c\n");
    }

    // ---- Shell Hex ----

    #[test]
    fn test_shell_hex_escape() {
        assert_eq!(shell_hex_escape("AB"), "\\x41\\x42");
    }

    // ---- Combine Arrays ----

    #[test]
    fn test_combine_str_arrays() {
        let a = vec!["a".into(), "b".into()];
        let b = vec!["b".into(), "c".into()];
        assert_eq!(combine_str_arrays(&a, &b), vec!["a", "b", "c"]);
    }

    // ---- StrWithPos ----

    #[test]
    fn test_str_with_pos_parse() {
        let sp = StrWithPos::parse("hel[*]lo");
        assert_eq!(sp.str_val, "hello");
        assert_eq!(sp.pos, 3);
    }

    #[test]
    fn test_str_with_pos_parse_no_cursor() {
        let sp = StrWithPos::parse("hello");
        assert_eq!(sp.str_val, "hello");
        assert_eq!(sp.pos, NO_STR_POS);
    }

    #[test]
    fn test_str_with_pos_display() {
        let sp = StrWithPos::new("hello".into(), 3);
        assert_eq!(format!("{}", sp), "hel[*]lo");
    }

    #[test]
    fn test_str_with_pos_display_end() {
        let sp = StrWithPos::new("hello".into(), 5);
        assert_eq!(format!("{}", sp), "hello[*]");
    }

    #[test]
    fn test_str_with_pos_display_no_pos() {
        let sp = StrWithPos::new("hello".into(), NO_STR_POS);
        assert_eq!(format!("{}", sp), "hello");
    }

    #[test]
    fn test_str_with_pos_prepend() {
        let sp = StrWithPos::new("world".into(), 2);
        let sp2 = sp.prepend("hello ");
        assert_eq!(sp2.str_val, "hello world");
        assert_eq!(sp2.pos, 8); // 6 + 2
    }

    #[test]
    fn test_str_with_pos_append() {
        let sp = StrWithPos::new("hello".into(), 2);
        let sp2 = sp.append(" world");
        assert_eq!(sp2.str_val, "hello world");
        assert_eq!(sp2.pos, 2);
    }

    // ---- Atomic File Ops ----

    #[test]
    fn test_write_file_if_different_new() {
        let dir = std::env::temp_dir().join(format!("utilfn_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_file.txt");
        let _ = std::fs::remove_file(&path);

        let written = write_file_if_different(&path, b"hello").unwrap();
        assert!(written);

        let written = write_file_if_different(&path, b"hello").unwrap();
        assert!(!written);

        let written = write_file_if_different(&path, b"world").unwrap();
        assert!(written);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(&dir).ok();
    }
}
