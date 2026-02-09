// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Telemetry: activity tracking, event recording, and upload management.
//! Port of Go's pkg/telemetry/ and pkg/telemetry/telemetrydata/.

use crate::backend::daystr;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

// ---- Constants ----

pub const MAX_TZ_NAME_LEN: usize = 50;
pub const ACTIVITY_EVENT_NAME: &str = "app:activity";
pub const MAX_ACTIVITY_DAYS: i64 = 30;
pub const MAX_TEVENT_AGE_DAYS: i64 = 28;

/// Valid telemetry event names.
pub const VALID_EVENT_NAMES: &[&str] = &[
    "app:activity",
    "app:navigate",
    "app:display",
    "action:blockclose",
    "action:magnify",
    "action:settheme",
    "action:newtab",
    "action:settabtheme",
    "action:onboarding",
    "action:codecopy",
    "conn:connectremote",
    "debug:panic",
    "wsh:run",
    "waveai:request",
    "waveai:feedback",
];

// ---- TEvent Types (from telemetrydata) ----

/// User-level properties attached to telemetry events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TEventUserProps {
    #[serde(rename = "client:arch", skip_serializing_if = "Option::is_none")]
    pub client_arch: Option<String>,
    #[serde(rename = "client:version", skip_serializing_if = "Option::is_none")]
    pub client_version: Option<String>,
    #[serde(rename = "client:initial_version", skip_serializing_if = "Option::is_none")]
    pub client_initial_version: Option<String>,
    #[serde(rename = "client:buildtime", skip_serializing_if = "Option::is_none")]
    pub client_build_time: Option<String>,
    #[serde(rename = "client:osrelease", skip_serializing_if = "Option::is_none")]
    pub client_os_release: Option<String>,
    #[serde(rename = "client:isdev", skip_serializing_if = "Option::is_none")]
    pub client_is_dev: Option<bool>,
    #[serde(rename = "autoupdate:channel", skip_serializing_if = "Option::is_none")]
    pub auto_update_channel: Option<String>,
    #[serde(rename = "autoupdate:enabled", skip_serializing_if = "Option::is_none")]
    pub auto_update_enabled: Option<bool>,
    #[serde(rename = "localshell:type", skip_serializing_if = "Option::is_none")]
    pub local_shell_type: Option<String>,
    #[serde(rename = "localshell:version", skip_serializing_if = "Option::is_none")]
    pub local_shell_version: Option<String>,
    #[serde(rename = "loc:countrycode", skip_serializing_if = "Option::is_none")]
    pub loc_country_code: Option<String>,
    #[serde(rename = "loc:regioncode", skip_serializing_if = "Option::is_none")]
    pub loc_region_code: Option<String>,
    #[serde(rename = "settings:customwidgets", skip_serializing_if = "Option::is_none")]
    pub settings_custom_widgets: Option<i32>,
    #[serde(rename = "settings:customaipresets", skip_serializing_if = "Option::is_none")]
    pub settings_custom_ai_presets: Option<i32>,
    #[serde(rename = "settings:customsettings", skip_serializing_if = "Option::is_none")]
    pub settings_custom_settings: Option<i32>,
}

/// Properties for telemetry events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TEventProps {
    // Activity counters
    #[serde(rename = "activity:activeminutes", skip_serializing_if = "Option::is_none")]
    pub active_minutes: Option<i32>,
    #[serde(rename = "activity:fgminutes", skip_serializing_if = "Option::is_none")]
    pub fg_minutes: Option<i32>,
    #[serde(rename = "activity:openminutes", skip_serializing_if = "Option::is_none")]
    pub open_minutes: Option<i32>,
    #[serde(rename = "activity:waveaiactiveminutes", skip_serializing_if = "Option::is_none")]
    pub wave_ai_active_minutes: Option<i32>,
    #[serde(rename = "activity:waveaifgminutes", skip_serializing_if = "Option::is_none")]
    pub wave_ai_fg_minutes: Option<i32>,

    // App state
    #[serde(rename = "app:firstday", skip_serializing_if = "Option::is_none")]
    pub app_first_day: Option<bool>,
    #[serde(rename = "app:firstlaunch", skip_serializing_if = "Option::is_none")]
    pub app_first_launch: Option<bool>,

    // Action tracking
    #[serde(rename = "action:initiator", skip_serializing_if = "Option::is_none")]
    pub action_initiator: Option<String>,

    // Debug info
    #[serde(rename = "debug:panictype", skip_serializing_if = "Option::is_none")]
    pub panic_type: Option<String>,

    // Block info
    #[serde(rename = "block:view", skip_serializing_if = "Option::is_none")]
    pub block_view: Option<String>,

    // AI tracking
    #[serde(rename = "ai:backendtype", skip_serializing_if = "Option::is_none")]
    pub ai_backend_type: Option<String>,
    #[serde(rename = "ai:local", skip_serializing_if = "Option::is_none")]
    pub ai_local: Option<bool>,

    // WSH command tracking
    #[serde(rename = "wsh:cmd", skip_serializing_if = "Option::is_none")]
    pub wsh_cmd: Option<String>,
    #[serde(rename = "wsh:haderror", skip_serializing_if = "Option::is_none")]
    pub wsh_had_error: Option<bool>,

    // Connection tracking
    #[serde(rename = "conn:conntype", skip_serializing_if = "Option::is_none")]
    pub conn_type: Option<String>,

    // Onboarding
    #[serde(rename = "onboarding:feature", skip_serializing_if = "Option::is_none")]
    pub onboarding_feature: Option<String>,
    #[serde(rename = "onboarding:version", skip_serializing_if = "Option::is_none")]
    pub onboarding_version: Option<String>,
    #[serde(rename = "onboarding:githubstar", skip_serializing_if = "Option::is_none")]
    pub onboarding_github_star: Option<String>,

    // Display info
    #[serde(rename = "display:height", skip_serializing_if = "Option::is_none")]
    pub display_height: Option<i32>,
    #[serde(rename = "display:width", skip_serializing_if = "Option::is_none")]
    pub display_width: Option<i32>,
    #[serde(rename = "display:dpr", skip_serializing_if = "Option::is_none")]
    pub display_dpr: Option<f64>,
    #[serde(rename = "display:count", skip_serializing_if = "Option::is_none")]
    pub display_count: Option<i32>,
    #[serde(rename = "display:all", skip_serializing_if = "Option::is_none")]
    pub display_all: Option<serde_json::Value>,

    // Count metrics
    #[serde(rename = "count:blocks", skip_serializing_if = "Option::is_none")]
    pub count_blocks: Option<i32>,
    #[serde(rename = "count:tabs", skip_serializing_if = "Option::is_none")]
    pub count_tabs: Option<i32>,
    #[serde(rename = "count:windows", skip_serializing_if = "Option::is_none")]
    pub count_windows: Option<i32>,
    #[serde(rename = "count:workspaces", skip_serializing_if = "Option::is_none")]
    pub count_workspaces: Option<i32>,
    #[serde(rename = "count:sshconn", skip_serializing_if = "Option::is_none")]
    pub count_ssh_conn: Option<i32>,
    #[serde(rename = "count:wslconn", skip_serializing_if = "Option::is_none")]
    pub count_wsl_conn: Option<i32>,
    #[serde(rename = "count:views", skip_serializing_if = "Option::is_none")]
    pub count_views: Option<HashMap<String, i32>>,

    // WaveAI metrics
    #[serde(rename = "waveai:apitype", skip_serializing_if = "Option::is_none")]
    pub wave_ai_api_type: Option<String>,
    #[serde(rename = "waveai:model", skip_serializing_if = "Option::is_none")]
    pub wave_ai_model: Option<String>,
    #[serde(rename = "waveai:inputtokens", skip_serializing_if = "Option::is_none")]
    pub wave_ai_input_tokens: Option<i32>,
    #[serde(rename = "waveai:outputtokens", skip_serializing_if = "Option::is_none")]
    pub wave_ai_output_tokens: Option<i32>,
    #[serde(rename = "waveai:nativewebsearchcount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_native_web_search_count: Option<i32>,
    #[serde(rename = "waveai:requestcount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_request_count: Option<i32>,
    #[serde(rename = "waveai:toolusecount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_tool_use_count: Option<i32>,
    #[serde(rename = "waveai:tooluseerrorcount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_tool_use_error_count: Option<i32>,
    #[serde(rename = "waveai:tooldetail", skip_serializing_if = "Option::is_none")]
    pub wave_ai_tool_detail: Option<HashMap<String, i32>>,
    #[serde(rename = "waveai:premiumreq", skip_serializing_if = "Option::is_none")]
    pub wave_ai_premium_req: Option<i32>,
    #[serde(rename = "waveai:proxyreq", skip_serializing_if = "Option::is_none")]
    pub wave_ai_proxy_req: Option<i32>,
    #[serde(rename = "waveai:haderror", skip_serializing_if = "Option::is_none")]
    pub wave_ai_had_error: Option<bool>,
    #[serde(rename = "waveai:imagecount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_image_count: Option<i32>,
    #[serde(rename = "waveai:pdfcount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_pdf_count: Option<i32>,
    #[serde(rename = "waveai:textdoccount", skip_serializing_if = "Option::is_none")]
    pub wave_ai_text_doc_count: Option<i32>,
    #[serde(rename = "waveai:textlen", skip_serializing_if = "Option::is_none")]
    pub wave_ai_text_len: Option<i32>,
    #[serde(rename = "waveai:firstbytems", skip_serializing_if = "Option::is_none")]
    pub wave_ai_first_byte_ms: Option<i32>,
    #[serde(rename = "waveai:requestdurms", skip_serializing_if = "Option::is_none")]
    pub wave_ai_request_dur_ms: Option<i32>,
    #[serde(rename = "waveai:widgetaccess", skip_serializing_if = "Option::is_none")]
    pub wave_ai_widget_access: Option<bool>,
    #[serde(rename = "waveai:feedback", skip_serializing_if = "Option::is_none")]
    pub wave_ai_feedback: Option<String>,

    // User properties (for $set / $set_once in analytics)
    #[serde(rename = "$set", skip_serializing_if = "Option::is_none")]
    pub user_set: Option<TEventUserProps>,
    #[serde(rename = "$set_once", skip_serializing_if = "Option::is_none")]
    pub user_set_once: Option<TEventUserProps>,
}

/// A single telemetry event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    /// ISO 8601 local timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tslocal: Option<String>,
    pub event: String,
    pub props: TEventProps,
    /// Whether this event has been uploaded (not serialized to JSON).
    #[serde(skip)]
    pub uploaded: bool,
}

impl TEvent {
    /// Create a new TEvent with UUID and current timestamp.
    pub fn new(event: &str, props: TEventProps) -> Self {
        let now = chrono::Utc::now();
        let local = chrono::Local::now();
        TEvent {
            uuid: Some(uuid::Uuid::new_v4().to_string()),
            ts: Some(now.timestamp_millis()),
            tslocal: Some(local.format("%Y-%m-%dT%H:%M:%S").to_string()),
            event: event.to_string(),
            props,
            uploaded: false,
        }
    }

    /// Ensure timestamps are set.
    pub fn ensure_timestamps(&mut self) {
        if self.ts.is_none() {
            self.ts = Some(chrono::Utc::now().timestamp_millis());
        }
        if self.tslocal.is_none() {
            self.tslocal = Some(chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string());
        }
    }

    /// Get or create user set props.
    pub fn user_set_props(&mut self) -> &mut TEventUserProps {
        self.props.user_set.get_or_insert_with(TEventUserProps::default)
    }

    /// Get or create user set-once props.
    pub fn user_set_once_props(&mut self) -> &mut TEventUserProps {
        self.props.user_set_once.get_or_insert_with(TEventUserProps::default)
    }

    /// Validate the event.
    pub fn validate(&self, current: bool) -> Result<(), String> {
        if self.event.is_empty() {
            return Err("event name is empty".to_string());
        }
        if !VALID_EVENT_NAMES.contains(&self.event.as_str()) {
            return Err(format!("invalid event name: {}", self.event));
        }
        if let Some(ref uuid) = self.uuid {
            if uuid.is_empty() {
                return Err("uuid is empty".to_string());
            }
        }
        if current {
            if let Some(ts) = self.ts {
                let now = chrono::Utc::now().timestamp_millis();
                let diff = (now - ts).abs();
                // Allow 5 minute skew
                if diff > 5 * 60 * 1000 {
                    return Err(format!("timestamp too far from current time: diff={}ms", diff));
                }
            }
        }
        Ok(())
    }
}

// ---- Activity Types (from telemetry.go) ----

/// Display information for activity tracking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityDisplayType {
    pub width: i32,
    pub height: i32,
    pub dpr: f64,
    pub internal: bool,
}

/// Daily telemetry data aggregation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TelemetryData {
    #[serde(rename = "activeminutes")]
    pub active_minutes: i32,
    #[serde(rename = "fgminutes")]
    pub fg_minutes: i32,
    #[serde(rename = "openminutes")]
    pub open_minutes: i32,
    #[serde(rename = "waveaiactiveminutes", skip_serializing_if = "is_zero")]
    pub wave_ai_active_minutes: i32,
    #[serde(rename = "waveaifgminutes", skip_serializing_if = "is_zero")]
    pub wave_ai_fg_minutes: i32,
    #[serde(rename = "numtabs")]
    pub num_tabs: i32,
    #[serde(rename = "numblocks", skip_serializing_if = "is_zero")]
    pub num_blocks: i32,
    #[serde(rename = "numwindows", skip_serializing_if = "is_zero")]
    pub num_windows: i32,
    #[serde(rename = "numws", skip_serializing_if = "is_zero")]
    pub num_ws: i32,
    #[serde(rename = "numwsnamed", skip_serializing_if = "is_zero")]
    pub num_ws_named: i32,
    #[serde(rename = "numsshconn", skip_serializing_if = "is_zero")]
    pub num_ssh_conn: i32,
    #[serde(rename = "numwslconn", skip_serializing_if = "is_zero")]
    pub num_wsl_conn: i32,
    #[serde(rename = "nummagnify", skip_serializing_if = "is_zero")]
    pub num_magnify: i32,
    #[serde(rename = "newtab")]
    pub new_tab: i32,
    #[serde(rename = "numstartup", skip_serializing_if = "is_zero")]
    pub num_startup: i32,
    #[serde(rename = "numshutdown", skip_serializing_if = "is_zero")]
    pub num_shutdown: i32,
    #[serde(rename = "numpanics", skip_serializing_if = "is_zero")]
    pub num_panics: i32,
    #[serde(rename = "numaireqs", skip_serializing_if = "is_zero")]
    pub num_ai_reqs: i32,
    #[serde(rename = "settabtheme", skip_serializing_if = "is_zero")]
    pub set_tab_theme: i32,
    #[serde(rename = "displays", skip_serializing_if = "Option::is_none")]
    pub displays: Option<Vec<ActivityDisplayType>>,
    #[serde(rename = "renderers", skip_serializing_if = "Option::is_none")]
    pub renderers: Option<HashMap<String, i32>>,
    #[serde(rename = "blocks", skip_serializing_if = "Option::is_none")]
    pub blocks: Option<HashMap<String, i32>>,
    #[serde(rename = "wshcmds", skip_serializing_if = "Option::is_none")]
    pub wsh_cmds: Option<HashMap<String, i32>>,
    #[serde(rename = "conn", skip_serializing_if = "Option::is_none")]
    pub conn: Option<HashMap<String, i32>>,
}

fn is_zero(val: &i32) -> bool {
    *val == 0
}

/// Daily activity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityType {
    pub day: String,
    #[serde(skip)]
    pub uploaded: bool,
    pub tdata: TelemetryData,
    pub tzname: String,
    pub tzoffset: i32,
    pub clientversion: String,
    pub clientarch: String,
    pub buildtime: String,
    pub osrelease: String,
}

impl ActivityType {
    /// Create a new activity record for today.
    pub fn new_today() -> Self {
        ActivityType {
            day: daystr::get_cur_day_str(),
            uploaded: false,
            tdata: TelemetryData::default(),
            tzname: String::new(),
            tzoffset: 0,
            clientversion: String::new(),
            clientarch: String::new(),
            buildtime: String::new(),
            osrelease: String::new(),
        }
    }
}

// ---- Activity Update (matches wshrpc.ActivityUpdate) ----

/// Incremental activity update sent from the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityUpdate {
    #[serde(rename = "fgminutes", skip_serializing_if = "Option::is_none")]
    pub fg_minutes: Option<i32>,
    #[serde(rename = "activeminutes", skip_serializing_if = "Option::is_none")]
    pub active_minutes: Option<i32>,
    #[serde(rename = "openminutes", skip_serializing_if = "Option::is_none")]
    pub open_minutes: Option<i32>,
    #[serde(rename = "waveaiactiveminutes", skip_serializing_if = "Option::is_none")]
    pub wave_ai_active_minutes: Option<i32>,
    #[serde(rename = "waveaifgminutes", skip_serializing_if = "Option::is_none")]
    pub wave_ai_fg_minutes: Option<i32>,
    #[serde(rename = "newtab", skip_serializing_if = "Option::is_none")]
    pub new_tab: Option<i32>,
    #[serde(rename = "nummagnify", skip_serializing_if = "Option::is_none")]
    pub num_magnify: Option<i32>,
    #[serde(rename = "settabtheme", skip_serializing_if = "Option::is_none")]
    pub set_tab_theme: Option<i32>,
    #[serde(rename = "numaireqs", skip_serializing_if = "Option::is_none")]
    pub num_ai_reqs: Option<i32>,
    #[serde(rename = "renderers", skip_serializing_if = "Option::is_none")]
    pub renderers: Option<HashMap<String, i32>>,
    #[serde(rename = "blocks", skip_serializing_if = "Option::is_none")]
    pub blocks: Option<HashMap<String, i32>>,
    #[serde(rename = "wshcmds", skip_serializing_if = "Option::is_none")]
    pub wsh_cmds: Option<HashMap<String, i32>>,
    #[serde(rename = "conn", skip_serializing_if = "Option::is_none")]
    pub conn: Option<HashMap<String, i32>>,
    #[serde(rename = "displays", skip_serializing_if = "Option::is_none")]
    pub displays: Option<Vec<ActivityDisplayType>>,
}

// ---- Telemetry Store ----

/// Cached ToS agreed timestamp.
static TOS_AGREED_TS: AtomicI64 = AtomicI64::new(0);

/// Get the ToS agreed timestamp (cached).
pub fn get_tos_agreed_ts() -> i64 {
    TOS_AGREED_TS.load(Ordering::Relaxed)
}

/// Set the ToS agreed timestamp (called after reading from DB).
pub fn set_tos_agreed_ts(ts: i64) {
    TOS_AGREED_TS.store(ts, Ordering::Relaxed);
}

/// In-memory telemetry store for activity tracking.
/// In production, this will be backed by SQLite.
pub struct TelemetryStore {
    /// Current day's activity record.
    current_activity: RwLock<ActivityType>,
    /// Recorded telemetry events (in-memory buffer).
    events: Mutex<Vec<TEvent>>,
}

impl Default for TelemetryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryStore {
    /// Create a new telemetry store.
    pub fn new() -> Self {
        TelemetryStore {
            current_activity: RwLock::new(ActivityType::new_today()),
            events: Mutex::new(Vec::new()),
        }
    }

    /// Record a telemetry event.
    pub fn record_event(&self, event: TEvent) -> Result<(), String> {
        event.validate(true)?;
        let mut events = self.events.lock().unwrap();
        events.push(event);
        Ok(())
    }

    /// Apply an activity update to the current day's record.
    pub fn update_activity(&self, update: &ActivityUpdate) {
        let mut activity = self.current_activity.write().unwrap();

        // Roll over to new day if needed
        let today = daystr::get_cur_day_str();
        if activity.day != today {
            // Reset for new day
            *activity = ActivityType::new_today();
        }

        // Accumulate counters
        if let Some(v) = update.fg_minutes {
            activity.tdata.fg_minutes += v;
        }
        if let Some(v) = update.active_minutes {
            activity.tdata.active_minutes += v;
        }
        if let Some(v) = update.open_minutes {
            activity.tdata.open_minutes += v;
        }
        if let Some(v) = update.wave_ai_active_minutes {
            activity.tdata.wave_ai_active_minutes += v;
        }
        if let Some(v) = update.wave_ai_fg_minutes {
            activity.tdata.wave_ai_fg_minutes += v;
        }
        if let Some(v) = update.new_tab {
            activity.tdata.new_tab += v;
        }
        if let Some(v) = update.num_magnify {
            activity.tdata.num_magnify += v;
        }
        if let Some(v) = update.set_tab_theme {
            activity.tdata.set_tab_theme += v;
        }
        if let Some(v) = update.num_ai_reqs {
            activity.tdata.num_ai_reqs += v;
        }

        // Merge map counters
        if let Some(ref renderers) = update.renderers {
            let map = activity.tdata.renderers.get_or_insert_with(HashMap::new);
            for (k, v) in renderers {
                *map.entry(k.clone()).or_insert(0) += v;
            }
        }
        if let Some(ref blocks) = update.blocks {
            let map = activity.tdata.blocks.get_or_insert_with(HashMap::new);
            for (k, v) in blocks {
                *map.entry(k.clone()).or_insert(0) += v;
            }
        }
        if let Some(ref wsh_cmds) = update.wsh_cmds {
            let map = activity.tdata.wsh_cmds.get_or_insert_with(HashMap::new);
            for (k, v) in wsh_cmds {
                *map.entry(k.clone()).or_insert(0) += v;
            }
        }
        if let Some(ref conn) = update.conn {
            let map = activity.tdata.conn.get_or_insert_with(HashMap::new);
            for (k, v) in conn {
                *map.entry(k.clone()).or_insert(0) += v;
            }
        }

        // Replace displays (last update wins)
        if let Some(ref displays) = update.displays {
            activity.tdata.displays = Some(displays.clone());
        }
    }

    /// Get a snapshot of the current activity.
    pub fn get_current_activity(&self) -> ActivityType {
        self.current_activity.read().unwrap().clone()
    }

    /// Get all non-uploaded events (draining the buffer).
    pub fn take_events(&self) -> Vec<TEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }

    /// Get the count of buffered events.
    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

/// Thread-safe shared telemetry store.
pub type SharedTelemetryStore = Arc<TelemetryStore>;

/// Create a new shared telemetry store.
pub fn new_shared_store() -> SharedTelemetryStore {
    Arc::new(TelemetryStore::new())
}

// ---- Merge helpers ----

/// Merge a map of counters (used for renderers, blocks, wsh_cmds, conn).
pub fn merge_counter_map(
    target: &mut Option<HashMap<String, i32>>,
    source: &Option<HashMap<String, i32>>,
) {
    if let Some(ref src) = source {
        let map = target.get_or_insert_with(HashMap::new);
        for (k, v) in src {
            *map.entry(k.clone()).or_insert(0) += v;
        }
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tevent_new() {
        let event = TEvent::new("app:activity", TEventProps::default());
        assert_eq!(event.event, "app:activity");
        assert!(event.uuid.is_some());
        assert!(event.ts.is_some());
        assert!(event.tslocal.is_some());
        assert!(!event.uploaded);
    }

    #[test]
    fn test_tevent_validate_valid() {
        let event = TEvent::new("app:activity", TEventProps::default());
        assert!(event.validate(true).is_ok());
    }

    #[test]
    fn test_tevent_validate_invalid_name() {
        let event = TEvent::new("invalid:event", TEventProps::default());
        assert!(event.validate(false).is_err());
    }

    #[test]
    fn test_tevent_validate_empty_name() {
        let event = TEvent {
            uuid: Some("test".to_string()),
            ts: Some(0),
            tslocal: None,
            event: String::new(),
            props: TEventProps::default(),
            uploaded: false,
        };
        assert!(event.validate(false).is_err());
    }

    #[test]
    fn test_tevent_ensure_timestamps() {
        let mut event = TEvent {
            uuid: None,
            ts: None,
            tslocal: None,
            event: "app:activity".to_string(),
            props: TEventProps::default(),
            uploaded: false,
        };
        event.ensure_timestamps();
        assert!(event.ts.is_some());
        assert!(event.tslocal.is_some());
    }

    #[test]
    fn test_tevent_user_set_props() {
        let mut event = TEvent::new("app:activity", TEventProps::default());
        assert!(event.props.user_set.is_none());
        event.user_set_props().client_arch = Some("linux/amd64".to_string());
        assert!(event.props.user_set.is_some());
        assert_eq!(
            event.props.user_set.as_ref().unwrap().client_arch.as_deref(),
            Some("linux/amd64")
        );
    }

    #[test]
    fn test_tevent_serde_roundtrip() {
        let mut props = TEventProps::default();
        props.active_minutes = Some(5);
        props.panic_type = Some("test-panic".to_string());
        let event = TEvent::new("app:activity", props);

        let json = serde_json::to_string(&event).unwrap();
        let deser: TEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.event, "app:activity");
        assert_eq!(deser.props.active_minutes, Some(5));
        assert_eq!(deser.props.panic_type.as_deref(), Some("test-panic"));
    }

    #[test]
    fn test_tevent_json_tags() {
        let mut props = TEventProps::default();
        props.wave_ai_api_type = Some("anthropic".to_string());
        props.count_blocks = Some(10);
        let event = TEvent::new("app:activity", props);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""waveai:apitype":"anthropic""#));
        assert!(json.contains(r#""count:blocks":10"#));
    }

    #[test]
    fn test_tevent_skip_none_fields() {
        let event = TEvent::new("app:activity", TEventProps::default());
        let json = serde_json::to_string(&event).unwrap();
        // None fields should be omitted
        assert!(!json.contains("waveai:apitype"));
        assert!(!json.contains("count:blocks"));
        assert!(!json.contains("$set"));
    }

    #[test]
    fn test_telemetry_data_serde() {
        let tdata = TelemetryData {
            active_minutes: 30,
            fg_minutes: 20,
            open_minutes: 60,
            num_tabs: 5,
            new_tab: 2,
            ..Default::default()
        };

        let json = serde_json::to_string(&tdata).unwrap();
        assert!(json.contains(r#""activeminutes":30"#));
        assert!(json.contains(r#""fgminutes":20"#));
        assert!(json.contains(r#""numtabs":5"#));
        // Zero fields with skip_serializing_if should be omitted
        assert!(!json.contains("numpanics"));
        assert!(!json.contains("numsshconn"));
    }

    #[test]
    fn test_telemetry_data_go_wire_compat() {
        let json = r#"{"activeminutes":10,"fgminutes":5,"openminutes":30,"numtabs":3,"newtab":1,"numblocks":8,"numsshconn":2,"renderers":{"webgl":5,"canvas":1}}"#;
        let tdata: TelemetryData = serde_json::from_str(json).unwrap();
        assert_eq!(tdata.active_minutes, 10);
        assert_eq!(tdata.fg_minutes, 5);
        assert_eq!(tdata.num_tabs, 3);
        assert_eq!(tdata.num_blocks, 8);
        assert_eq!(tdata.num_ssh_conn, 2);
        let renderers = tdata.renderers.as_ref().unwrap();
        assert_eq!(renderers["webgl"], 5);
        assert_eq!(renderers["canvas"], 1);
    }

    #[test]
    fn test_activity_type_new_today() {
        let activity = ActivityType::new_today();
        assert_eq!(activity.day, daystr::get_cur_day_str());
        assert!(!activity.uploaded);
        assert_eq!(activity.tdata.active_minutes, 0);
    }

    #[test]
    fn test_activity_update_serde() {
        let json = r#"{"fgminutes":1,"activeminutes":1,"newtab":2}"#;
        let update: ActivityUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.fg_minutes, Some(1));
        assert_eq!(update.active_minutes, Some(1));
        assert_eq!(update.new_tab, Some(2));
        assert_eq!(update.open_minutes, None);
    }

    #[test]
    fn test_activity_display_type_serde() {
        let display = ActivityDisplayType {
            width: 1920,
            height: 1080,
            dpr: 2.0,
            internal: true,
        };
        let json = serde_json::to_string(&display).unwrap();
        let deser: ActivityDisplayType = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.width, 1920);
        assert_eq!(deser.height, 1080);
        assert!((deser.dpr - 2.0).abs() < f64::EPSILON);
        assert!(deser.internal);
    }

    #[test]
    fn test_telemetry_store_record_event() {
        let store = TelemetryStore::new();
        let event = TEvent::new("app:activity", TEventProps::default());
        assert!(store.record_event(event).is_ok());
        assert_eq!(store.event_count(), 1);
    }

    #[test]
    fn test_telemetry_store_invalid_event() {
        let store = TelemetryStore::new();
        let event = TEvent::new("invalid:name", TEventProps::default());
        assert!(store.record_event(event).is_err());
        assert_eq!(store.event_count(), 0);
    }

    #[test]
    fn test_telemetry_store_take_events() {
        let store = TelemetryStore::new();
        store
            .record_event(TEvent::new("app:activity", TEventProps::default()))
            .unwrap();
        store
            .record_event(TEvent::new("app:navigate", TEventProps::default()))
            .unwrap();
        assert_eq!(store.event_count(), 2);

        let events = store.take_events();
        assert_eq!(events.len(), 2);
        assert_eq!(store.event_count(), 0); // buffer drained
    }

    #[test]
    fn test_telemetry_store_update_activity() {
        let store = TelemetryStore::new();

        let update = ActivityUpdate {
            fg_minutes: Some(5),
            active_minutes: Some(3),
            new_tab: Some(2),
            ..Default::default()
        };
        store.update_activity(&update);

        let activity = store.get_current_activity();
        assert_eq!(activity.tdata.fg_minutes, 5);
        assert_eq!(activity.tdata.active_minutes, 3);
        assert_eq!(activity.tdata.new_tab, 2);

        // Accumulate another update
        let update2 = ActivityUpdate {
            fg_minutes: Some(3),
            new_tab: Some(1),
            ..Default::default()
        };
        store.update_activity(&update2);

        let activity = store.get_current_activity();
        assert_eq!(activity.tdata.fg_minutes, 8); // 5 + 3
        assert_eq!(activity.tdata.new_tab, 3); // 2 + 1
    }

    #[test]
    fn test_telemetry_store_update_activity_maps() {
        let store = TelemetryStore::new();

        let mut renderers = HashMap::new();
        renderers.insert("webgl".to_string(), 3);
        renderers.insert("canvas".to_string(), 1);

        let update = ActivityUpdate {
            renderers: Some(renderers),
            ..Default::default()
        };
        store.update_activity(&update);

        let mut more_renderers = HashMap::new();
        more_renderers.insert("webgl".to_string(), 2);
        more_renderers.insert("dom".to_string(), 1);

        let update2 = ActivityUpdate {
            renderers: Some(more_renderers),
            ..Default::default()
        };
        store.update_activity(&update2);

        let activity = store.get_current_activity();
        let renderers = activity.tdata.renderers.unwrap();
        assert_eq!(renderers["webgl"], 5); // 3 + 2
        assert_eq!(renderers["canvas"], 1);
        assert_eq!(renderers["dom"], 1);
    }

    #[test]
    fn test_tos_agreed_ts() {
        set_tos_agreed_ts(1700000000);
        assert_eq!(get_tos_agreed_ts(), 1700000000);
    }

    #[test]
    fn test_merge_counter_map() {
        let mut target: Option<HashMap<String, i32>> = None;

        let mut src = HashMap::new();
        src.insert("a".to_string(), 5);
        merge_counter_map(&mut target, &Some(src));
        assert_eq!(target.as_ref().unwrap()["a"], 5);

        let mut src2 = HashMap::new();
        src2.insert("a".to_string(), 3);
        src2.insert("b".to_string(), 1);
        merge_counter_map(&mut target, &Some(src2));
        assert_eq!(target.as_ref().unwrap()["a"], 8);
        assert_eq!(target.as_ref().unwrap()["b"], 1);

        // Merging None should be a no-op
        merge_counter_map(&mut target, &None);
        assert_eq!(target.as_ref().unwrap()["a"], 8);
    }

    #[test]
    fn test_shared_store() {
        let store = new_shared_store();
        let store2 = store.clone();

        store
            .record_event(TEvent::new("app:activity", TEventProps::default()))
            .unwrap();
        assert_eq!(store2.event_count(), 1); // shared access
    }

    #[test]
    fn test_tevent_props_go_wire_compat() {
        // Test that Go-generated JSON deserializes correctly
        let json = r##"{"activity:activeminutes":15,"activity:fgminutes":10,"debug:panictype":"nil-ptr","waveai:apitype":"anthropic","waveai:model":"claude-3","count:blocks":42,"count:views":{"term":5,"web":3}}"##;
        let props: TEventProps = serde_json::from_str(json).unwrap();
        assert_eq!(props.active_minutes, Some(15));
        assert_eq!(props.fg_minutes, Some(10));
        assert_eq!(props.panic_type.as_deref(), Some("nil-ptr"));
        assert_eq!(props.wave_ai_api_type.as_deref(), Some("anthropic"));
        assert_eq!(props.wave_ai_model.as_deref(), Some("claude-3"));
        assert_eq!(props.count_blocks, Some(42));
        let views = props.count_views.unwrap();
        assert_eq!(views["term"], 5);
        assert_eq!(views["web"], 3);
    }

    #[test]
    fn test_user_props_go_wire_compat() {
        let json = r#"{"client:arch":"linux/amd64","client:version":"0.19.4","autoupdate:enabled":true,"settings:customwidgets":3}"#;
        let props: TEventUserProps = serde_json::from_str(json).unwrap();
        assert_eq!(props.client_arch.as_deref(), Some("linux/amd64"));
        assert_eq!(props.client_version.as_deref(), Some("0.19.4"));
        assert_eq!(props.auto_update_enabled, Some(true));
        assert_eq!(props.settings_custom_widgets, Some(3));
    }

    #[test]
    fn test_activity_type_go_wire_compat() {
        let json = r#"{"day":"2024-03-15","tdata":{"activeminutes":30,"fgminutes":20,"openminutes":60,"numtabs":5,"newtab":2},"tzname":"America/Los_Angeles","tzoffset":-480,"clientversion":"0.19.4","clientarch":"linux/amd64","buildtime":"2024-03-15","osrelease":"Ubuntu 22.04"}"#;
        let activity: ActivityType = serde_json::from_str(json).unwrap();
        assert_eq!(activity.day, "2024-03-15");
        assert_eq!(activity.tdata.active_minutes, 30);
        assert_eq!(activity.tdata.fg_minutes, 20);
        assert_eq!(activity.tdata.num_tabs, 5);
        assert_eq!(activity.tzname, "America/Los_Angeles");
        assert_eq!(activity.tzoffset, -480);
        assert_eq!(activity.clientversion, "0.19.4");
    }

    #[test]
    fn test_valid_event_names() {
        assert!(VALID_EVENT_NAMES.contains(&"app:activity"));
        assert!(VALID_EVENT_NAMES.contains(&"debug:panic"));
        assert!(VALID_EVENT_NAMES.contains(&"waveai:request"));
        assert!(!VALID_EVENT_NAMES.contains(&"fake:event"));
    }
}
