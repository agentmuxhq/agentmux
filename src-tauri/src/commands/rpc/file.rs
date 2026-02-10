// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Wave file fetch handler.

use serde_json::Value;

use crate::state::AppState;

/// Fetch a wave file's data and metadata.
#[tauri::command(rename_all = "camelCase")]
pub async fn fetch_wave_file(
    zone_id: String,
    name: String,
    offset: Option<i64>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    use base64::Engine as _;
    let file_store = &state.file_store;

    let file_info = file_store
        .stat(&zone_id, &name)
        .map_err(|e| format!("stat: {}", e))?;

    let file_info = match file_info {
        Some(f) => f,
        None => {
            return Ok(serde_json::json!({
                "data": null,
                "fileInfo": null,
            }));
        }
    };

    let data_bytes = if let Some(off) = offset {
        let (_actual_offset, data) = file_store
            .read_at(&zone_id, &name, off, 0)
            .map_err(|e| format!("read_at: {}", e))?;
        data
    } else {
        file_store
            .read_file(&zone_id, &name)
            .map_err(|e| format!("read_file: {}", e))?
            .unwrap_or_default()
    };

    let data64 = base64::engine::general_purpose::STANDARD.encode(&data_bytes);
    let file_info_json =
        serde_json::to_value(&file_info).map_err(|e| format!("serialize file_info: {}", e))?;

    Ok(serde_json::json!({
        "data": data64,
        "fileInfo": file_info_json,
    }))
}
