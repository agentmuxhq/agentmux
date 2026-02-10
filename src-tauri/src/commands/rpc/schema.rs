// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! JSON schema delivery for Monaco editor validation.

use serde_json::Value;

const SCHEMA_SETTINGS: &str = include_str!("../../../../schema/settings.json");
const SCHEMA_CONNECTIONS: &str = include_str!("../../../../schema/connections.json");
const SCHEMA_AIPRESETS: &str = include_str!("../../../../schema/aipresets.json");
const SCHEMA_WIDGETS: &str = include_str!("../../../../schema/widgets.json");

/// Tauri command to deliver JSON schema by name.
#[tauri::command(rename_all = "camelCase")]
pub async fn get_schema(schema_name: String) -> Result<Value, String> {
    let json_str = match schema_name.as_str() {
        "settings" => SCHEMA_SETTINGS,
        "connections" => SCHEMA_CONNECTIONS,
        "aipresets" => SCHEMA_AIPRESETS,
        "widgets" => SCHEMA_WIDGETS,
        _ => return Err(format!("unknown schema: {}", schema_name)),
    };
    serde_json::from_str(json_str).map_err(|e| format!("parse schema {}: {}", schema_name, e))
}
