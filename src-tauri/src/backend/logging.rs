// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Enhanced logging framework with configurable levels and performance tracking.
//!
//! This module provides comprehensive logging capabilities for debugging
//! startup issues, window lifecycle, and performance bottlenecks.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;
use tracing::{info, Level};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Layer, Registry};

/// Global log level control (runtime adjustable)
static LOG_LEVEL: AtomicU8 = AtomicU8::new(3); // Default: INFO

/// Logging configuration
pub struct LogConfig {
    pub file_level: Level,
    pub console_level: Level,
    pub enable_structured: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file_level: Level::DEBUG, // More verbose in files
            console_level: Level::INFO, // Less verbose in console
            enable_structured: false,
        }
    }
}

/// Initialize enhanced logging system
pub fn init_logging_enhanced(config: LogConfig, log_dir: std::path::PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;

    // File appender with daily rotation
    let log_file = log_dir.join("agentmux.log");
    let file_appender =
        tracing_appender::rolling::daily(&log_dir, "agentmux.log");

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_filter(LevelFilter::from_level(config.file_level));

    // Console layer (stderr)
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_filter(LevelFilter::from_level(config.console_level));

    // Combine layers
    let subscriber = Registry::default()
        .with(file_layer)
        .with(console_layer);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| format!("Failed to set subscriber: {}", e))?;

    info!("🚀 Enhanced logging initialized");
    info!("📁 Log directory: {}", log_dir.display());
    info!("📊 File level: {:?}, Console level: {:?}", config.file_level, config.console_level);

    Ok(())
}


/// Runtime log level control
pub fn set_log_level(level: Level) {
    let level_u8 = match level {
        Level::ERROR => 1,
        Level::WARN => 2,
        Level::INFO => 3,
        Level::DEBUG => 4,
        Level::TRACE => 5,
    };
    LOG_LEVEL.store(level_u8, Ordering::Relaxed);
    info!("📝 Log level changed to: {:?}", level);
}

pub fn get_log_level() -> Level {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        1 => Level::ERROR,
        2 => Level::WARN,
        3 => Level::INFO,
        4 => Level::DEBUG,
        5 => Level::TRACE,
        _ => Level::INFO,
    }
}

/// Performance timer for measuring operation duration
pub struct PerfTimer {
    label: String,
    start: Instant,
}

impl PerfTimer {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        tracing::debug!("⏱️  START: {}", label);
        Self {
            label,
            start: Instant::now(),
        }
    }

    pub fn lap(&self, milestone: &str) {
        let elapsed = self.start.elapsed();
        tracing::debug!("⏱️  {} - {}: {:?}", self.label, milestone, elapsed);
    }

    pub fn finish(self) {
        let elapsed = self.start.elapsed();
        tracing::info!("✅ {} completed in {:?}", self.label, elapsed);
    }
}

impl Drop for PerfTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        tracing::debug!("⏱️  END: {} ({:?})", self.label, elapsed);
    }
}

/// Log window lifecycle event
pub fn log_window_event(window_label: &str, event: &str, details: Option<&str>) {
    if let Some(d) = details {
        tracing::info!("🪟  Window[{}]: {} - {}", window_label, event, d);
    } else {
        tracing::info!("🪟  Window[{}]: {}", window_label, event);
    }
}

/// Log startup milestone
pub fn log_startup_milestone(milestone: &str, elapsed_ms: u128) {
    tracing::info!("🚀 Startup milestone: {} (+{}ms)", milestone, elapsed_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_control() {
        set_log_level(Level::DEBUG);
        assert_eq!(get_log_level(), Level::DEBUG);

        set_log_level(Level::WARN);
        assert_eq!(get_log_level(), Level::WARN);
    }

    #[test]
    fn test_perf_timer() {
        let timer = PerfTimer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.lap("halfway");
        std::thread::sleep(std::time::Duration::from_millis(10));
        timer.finish();
    }
}
