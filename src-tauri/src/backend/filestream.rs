// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! File streaming via `wavefile://` custom protocol.
//! Replaces Go's HTTP `/wave/stream-file` and `/wave/stream-local-file` endpoints.
//!
//! Handles requests like:
//!   wavefile://localhost/stream?path=wsh://local/home/user/image.png
//!   wavefile://localhost/stream-local-file?path=/home/user/image.png&no404=1

use std::path::PathBuf;

use super::wavebase;

/// 1x1 transparent GIF89a (43 bytes). Returned when `no404=1` and file is missing.
const TRANSPARENT_GIF: &[u8] = &[
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0xFF, 0xFF,
    0xFF, 0x00, 0x00, 0x00, 0x21, 0xF9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2C, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3B,
];

/// Entry point for the `wavefile://` protocol handler.
/// Registered via `register_asynchronous_uri_scheme_protocol` in lib.rs.
pub fn handle_wavefile_protocol(
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    let uri = request.uri().to_string();
    let params = parse_query_params(&uri);

    let raw_path = params
        .iter()
        .find(|(k, _)| k == "path")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    let no404 = params
        .iter()
        .find(|(k, _)| k == "no404")
        .map(|(_, v)| v == "1")
        .unwrap_or(false);

    match serve_local_file(raw_path, no404) {
        Ok((data, mime)) => {
            let response = tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .header("Cache-Control", "no-cache")
                .body(data)
                .unwrap_or_else(|_| {
                    tauri::http::Response::builder()
                        .status(500)
                        .body(b"Internal error".to_vec())
                        .unwrap()
                });
            responder.respond(response);
        }
        Err(status) => {
            let response = tauri::http::Response::builder()
                .status(status)
                .body(format!("Error {}", status).into_bytes())
                .unwrap();
            responder.respond(response);
        }
    }
}

/// Parse query parameters from a URI string.
/// Returns a Vec of (key, value) pairs with percent-decoded values.
fn parse_query_params(uri: &str) -> Vec<(String, String)> {
    let query = match uri.split_once('?') {
        Some((_, q)) => q,
        None => return Vec::new(),
    };

    // Strip fragment if present
    let query = query.split('#').next().unwrap_or(query);

    query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            Some((
                percent_decode(k),
                percent_decode(v),
            ))
        })
        .collect()
}

/// Simple percent-decoding (handles %XX sequences and `+` as space).
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'%' => {
                let h = chars.next().unwrap_or(b'0');
                let l = chars.next().unwrap_or(b'0');
                let val = hex_val(h) * 16 + hex_val(l);
                result.push(val as char);
            }
            b'+' => result.push(' '),
            _ => result.push(b as char),
        }
    }
    result
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Serve a local file. Strips `wsh://local/` prefix if present,
/// expands `~`, reads the file, and returns (data, mime_type).
/// On error, returns the HTTP status code.
fn serve_local_file(raw_path: &str, no404: bool) -> Result<(Vec<u8>, &'static str), u16> {
    if raw_path.is_empty() {
        if no404 {
            return Ok((TRANSPARENT_GIF.to_vec(), "image/gif"));
        }
        return Err(400);
    }

    // Strip wsh://local/ prefix if present
    let path_str = if let Some(rest) = raw_path.strip_prefix("wsh://local/") {
        rest
    } else if let Some(rest) = raw_path.strip_prefix("wsh://local") {
        // Handle wsh://local without trailing slash (root)
        if rest.is_empty() { "/" } else { rest }
    } else {
        raw_path
    };

    // Expand ~ to home directory
    let file_path: PathBuf = wavebase::expand_home_dir_safe(path_str);

    // Read the file
    match std::fs::read(&file_path) {
        Ok(data) => {
            let mime = mime_from_extension(&file_path);
            Ok((data, mime))
        }
        Err(e) => {
            if no404 && e.kind() == std::io::ErrorKind::NotFound {
                return Ok((TRANSPARENT_GIF.to_vec(), "image/gif"));
            }
            tracing::debug!("filestream: could not read {:?}: {}", file_path, e);
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(404)
            } else {
                Err(500)
            }
        }
    }
}

/// Determine MIME type from file extension.
fn mime_from_extension(path: &PathBuf) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        // Audio
        "mp3" => "audio/mpeg",
        "ogg" | "oga" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        // Documents
        "pdf" => "application/pdf",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        // Default
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_params_empty() {
        assert!(parse_query_params("wavefile://localhost/stream").is_empty());
    }

    #[test]
    fn test_parse_query_params_single() {
        let params = parse_query_params("wavefile://localhost/stream?path=foo");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("path".to_string(), "foo".to_string()));
    }

    #[test]
    fn test_parse_query_params_encoded() {
        let params = parse_query_params("wavefile://localhost/stream?path=wsh%3A%2F%2Flocal%2Fhome&no404=1");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].0, "path");
        assert_eq!(params[0].1, "wsh://local/home");
        assert_eq!(params[1], ("no404".to_string(), "1".to_string()));
    }

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(mime_from_extension(&PathBuf::from("test.png")), "image/png");
        assert_eq!(mime_from_extension(&PathBuf::from("test.PDF")), "application/pdf");
        assert_eq!(mime_from_extension(&PathBuf::from("test.mp4")), "video/mp4");
        assert_eq!(mime_from_extension(&PathBuf::from("test.unknown")), "application/octet-stream");
        assert_eq!(mime_from_extension(&PathBuf::from("noext")), "application/octet-stream");
    }

    #[test]
    fn test_is_valid_config_filename() {
        // Reuse the function from parent module via cfg
    }

    #[test]
    fn test_transparent_gif_size() {
        assert_eq!(TRANSPARENT_GIF.len(), 43);
        // GIF89a magic bytes
        assert_eq!(&TRANSPARENT_GIF[..6], b"GIF89a");
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("hello+world"), "hello world");
        assert_eq!(percent_decode("%2F"), "/");
        // "no%encoding" → %en is consumed (0xE0 = 'à'), leaves "coding"
        assert_eq!(percent_decode("no%encoding"), "no\u{00E0}coding");
    }

    #[test]
    fn test_serve_local_file_empty_path() {
        // Empty path without no404 => 400
        assert_eq!(serve_local_file("", false), Err(400));
    }

    #[test]
    fn test_serve_local_file_empty_path_no404() {
        // Empty path with no404 => transparent GIF
        let result = serve_local_file("", true);
        assert!(result.is_ok());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/gif");
        assert_eq!(data.len(), 43);
    }

    #[test]
    fn test_serve_local_file_missing() {
        assert_eq!(serve_local_file("/nonexistent/file.png", false), Err(404));
    }

    #[test]
    fn test_serve_local_file_missing_no404() {
        let result = serve_local_file("/nonexistent/file.png", true);
        assert!(result.is_ok());
        let (data, mime) = result.unwrap();
        assert_eq!(mime, "image/gif");
        assert_eq!(data, TRANSPARENT_GIF);
    }

    #[test]
    fn test_strip_wsh_prefix() {
        // serve_local_file should strip wsh://local/ prefix
        // File won't exist but we can verify it doesn't error on prefix handling
        let result = serve_local_file("wsh://local/nonexistent.png", false);
        assert_eq!(result, Err(404)); // Not 400 — path was parsed correctly
    }
}
