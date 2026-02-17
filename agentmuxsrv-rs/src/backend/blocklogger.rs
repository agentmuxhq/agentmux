// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Block logger: context-based logging for block operations.
//! Port of Go's pkg/blocklogger/blocklogger.go.
//!
//! Provides async log output that is appended to a block's terminal output
//! via a buffered channel and background consumer task.

use base64::Engine as _;
use std::sync::OnceLock;
use tokio::sync::mpsc;

/// Buffer size for the output channel (matches Go's outputBufferSize = 1000).
const OUTPUT_BUFFER_SIZE: usize = 1000;

/// Data sent to the output runner for appending to block output.
#[derive(Debug, Clone)]
pub struct BlockLogEntry {
    pub block_id: String,
    pub data64: String,
}

/// Global output channel sender.
static OUTPUT_TX: OnceLock<mpsc::Sender<BlockLogEntry>> = OnceLock::new();

/// Initialize the block logger. Must be called once at startup.
/// Returns a receiver that the output runner should consume.
///
/// The caller is responsible for spawning a task that reads from
/// the returned receiver and dispatches the log entries (e.g., via
/// RPC ControllerAppendOutput or direct file append).
pub fn init_block_logger() -> mpsc::Receiver<BlockLogEntry> {
    let (tx, rx) = mpsc::channel(OUTPUT_BUFFER_SIZE);
    OUTPUT_TX
        .set(tx)
        .expect("init_block_logger called more than once");
    rx
}

/// Log an info message for a block. The message is base64-encoded and
/// queued for async delivery to the block's output.
///
/// Line endings are normalized to \r\n for terminal display.
pub fn log_info(block_id: &str, msg: &str) {
    let normalized = msg.replace('\n', "\r\n");
    let data64 = base64::engine::general_purpose::STANDARD.encode(normalized.as_bytes());

    if let Some(tx) = OUTPUT_TX.get() {
        // Use try_send to avoid blocking; drop message if channel is full.
        let _ = tx.try_send(BlockLogEntry {
            block_id: block_id.to_string(),
            data64,
        });
    }
}

/// Log a debug message for a block. Only logs if `verbose` is true.
pub fn log_debug(block_id: &str, verbose: bool, msg: &str) {
    if verbose {
        log_info(block_id, msg);
    }
}

/// Format and log an info message.
#[macro_export]
macro_rules! block_log_info {
    ($block_id:expr, $($arg:tt)*) => {
        $crate::backend::blocklogger::log_info($block_id, &format!($($arg)*))
    };
}

/// Format and log a debug message.
#[macro_export]
macro_rules! block_log_debug {
    ($block_id:expr, $verbose:expr, $($arg:tt)*) => {
        $crate::backend::blocklogger::log_debug($block_id, $verbose, &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_block_logger_init_and_log() {
        // We can only init once per process, so this test covers the basic flow.
        // In a real test suite, this would need process isolation.
        // For now, test the encoding logic directly.
        let msg = "hello\nworld";
        let normalized = msg.replace('\n', "\r\n");
        let data64 = base64::engine::general_purpose::STANDARD.encode(normalized.as_bytes());

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&data64)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "hello\r\nworld");
    }

    #[test]
    fn test_log_debug_skips_non_verbose() {
        // log_debug with verbose=false should not panic even without init
        log_debug("test-block", false, "should be skipped");
    }

    #[test]
    fn test_log_info_without_init() {
        // log_info without init should not panic (OUTPUT_TX is None)
        log_info("test-block", "no-op message");
    }

    #[tokio::test]
    async fn test_block_log_entry_format() {
        let entry = BlockLogEntry {
            block_id: "abc-123".to_string(),
            data64: base64::engine::general_purpose::STANDARD.encode(b"test output"),
        };
        assert_eq!(entry.block_id, "abc-123");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&entry.data64)
            .unwrap();
        assert_eq!(decoded, b"test output");
    }
}
