// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Database utilities for querying waveterm.db
// This is a temporary solution until proper RPC integration is implemented.

use std::path::PathBuf;

/// Query the waveterm.db for existing client/window/tab IDs
/// Returns (client_id, window_id, tab_id) or None if not found
pub fn get_existing_ids(data_dir: &PathBuf) -> Result<(String, String, String), String> {
    let db_path = data_dir.join("db").join("waveterm.db");

    if !db_path.exists() {
        return Err(format!("Database not found at {:?}", db_path));
    }

    // Use rusqlite to query the database
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    // Get client ID
    let client_id: String = conn
        .query_row("SELECT oid FROM db_client LIMIT 1", [], |row| row.get(0))
        .map_err(|e| format!("Failed to get client ID: {}", e))?;

    // Get window ID
    let window_id: String = conn
        .query_row("SELECT oid FROM db_window LIMIT 1", [], |row| row.get(0))
        .map_err(|e| format!("Failed to get window ID: {}", e))?;

    // Get tab ID
    let tab_id: String = conn
        .query_row("SELECT oid FROM db_tab LIMIT 1", [], |row| row.get(0))
        .map_err(|e| format!("Failed to get tab ID: {}", e))?;

    Ok((client_id, window_id, tab_id))
}
