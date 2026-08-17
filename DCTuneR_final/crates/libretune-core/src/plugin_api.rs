//! Plugin API surface - Host functions exposed to WASM plugins.
//!
//! Provides secure, permission-checked API for WASM plugins to:
//! - Read table and constant data
//! - Subscribe to realtime channels
//! - Log messages for debugging
//! - Execute action sequences
//!
//! All functions validate plugin permissions before executing.

use crate::plugin_system::{Permission, PluginDataSnapshot, PluginManager};
use std::sync::Mutex;

/// Prefix shared by every [`ApiResponse::permission_denied`] error, so
/// callers can distinguish "denied" from "not found"/other failures without
/// a separate status enum.
const PERMISSION_DENIED_PREFIX: &str = "Permission denied: ";

/// Plugin API context shared with WASM host functions.
pub struct PluginApiContext {
    /// Plugin manager reference
    pub plugin_manager: Mutex<PluginManager>,
}

impl PluginApiContext {
    /// Create new API context with plugin manager.
    pub fn new(plugin_manager: PluginManager) -> Self {
        PluginApiContext {
            plugin_manager: Mutex::new(plugin_manager),
        }
    }
}

/// Plugin API response for any function call.
#[derive(Debug, Clone)]
pub struct ApiResponse {
    /// Success or error
    pub success: bool,
    /// Optional data payload (as serialized bytes)
    pub data: Vec<u8>,
    /// Error message if failed
    pub error: String,
}

impl ApiResponse {
    /// Create successful response with data.
    pub fn ok(data: Vec<u8>) -> Self {
        ApiResponse {
            success: true,
            data,
            error: String::new(),
        }
    }

    /// Create success response with empty data.
    pub fn ok_empty() -> Self {
        ApiResponse {
            success: true,
            data: Vec::new(),
            error: String::new(),
        }
    }

    /// Create error response.
    pub fn error(msg: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: Vec::new(),
            error: msg.into(),
        }
    }

    /// Create permission denied response.
    pub fn permission_denied(perm_name: &str) -> Self {
        ApiResponse {
            success: false,
            data: Vec::new(),
            error: format!("{}{}", PERMISSION_DENIED_PREFIX, perm_name),
        }
    }

    /// Whether this failure was specifically a permission denial, as opposed
    /// to e.g. "table not found" or "cell out of range". Callers that need
    /// to map an `ApiResponse` back to a distinct wire-level error code (see
    /// `plugin_system::host_result`) use this instead of matching on the
    /// error string directly.
    pub fn is_permission_denied(&self) -> bool {
        !self.success && self.error.starts_with(PERMISSION_DENIED_PREFIX)
    }
}

/// Plugin log level for messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl LogLevel {
    /// Convert from numeric code.
    pub fn from_code(code: i32) -> Self {
        match code {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            _ => LogLevel::Error,
        }
    }

    /// Convert to string for display.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Plugin log message with timestamp.
#[derive(Debug, Clone)]
pub struct PluginLogMessage {
    /// Plugin name
    pub plugin_name: String,
    /// Log level
    pub level: LogLevel,
    /// Message text
    pub message: String,
    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: u64,
}

impl PluginLogMessage {
    /// Create new log message.
    pub fn new(
        plugin_name: impl Into<String>,
        level: LogLevel,
        message: impl Into<String>,
    ) -> Self {
        PluginLogMessage {
            plugin_name: plugin_name.into(),
            level,
            message: message.into(),
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    /// Format log message for display.
    pub fn format_display(&self) -> String {
        format!(
            "[{}] {} [{}]: {}",
            self.timestamp_ms,
            self.plugin_name,
            self.level.as_str(),
            self.message
        )
    }
}

/// Host function: Get table data (readonly).
///
/// # Permissions
/// Requires `ReadTables` permission.
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `snapshot` - Read-only tune/table data captured for this execute() run
/// * `table_name` - Name of table to read
/// * `row`, `col` - Zero-based cell coordinates into the table's z-values grid
///
/// # Returns
/// ApiResponse with the cell's value as little-endian f32 bytes, or an error
/// if the permission is missing, the table isn't in the snapshot, or the
/// coordinates are out of range.
pub fn api_get_table_data(
    granted: &[Permission],
    snapshot: &PluginDataSnapshot,
    table_name: &str,
    row: i32,
    col: i32,
) -> ApiResponse {
    if !granted.contains(&Permission::ReadTables) {
        return ApiResponse::permission_denied("ReadTables");
    }
    let Some(table) = snapshot.tables.get(table_name) else {
        return ApiResponse::error(format!("table '{}' not found", table_name));
    };
    if row < 0 || col < 0 {
        return ApiResponse::error("row/col must be non-negative");
    }
    match table
        .z_values
        .get(row as usize)
        .and_then(|r| r.get(col as usize))
    {
        Some(value) => ApiResponse::ok((*value as f32).to_le_bytes().to_vec()),
        None => ApiResponse::error(format!(
            "cell ({}, {}) out of range for table '{}'",
            row, col, table_name
        )),
    }
}

/// Host function: Get constant value.
///
/// # Permissions
/// Requires `ReadTables` permission (constants are table-like data).
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `snapshot` - Read-only tune/table data captured for this execute() run
/// * `constant_name` - Name of constant
///
/// # Returns
/// ApiResponse with the constant's scalar value as little-endian f32 bytes,
/// or an error if the permission is missing or the constant isn't a known
/// scalar in the snapshot (array-shaped constants, e.g. axis bins, aren't
/// exposed here — read them via the owning table instead).
pub fn api_get_constant(
    granted: &[Permission],
    snapshot: &PluginDataSnapshot,
    constant_name: &str,
) -> ApiResponse {
    if !granted.contains(&Permission::ReadTables) {
        return ApiResponse::permission_denied("ReadTables");
    }
    match snapshot.constants.get(constant_name) {
        Some(value) => ApiResponse::ok((*value as f32).to_le_bytes().to_vec()),
        None => ApiResponse::error(format!("constant '{}' not found", constant_name)),
    }
}

/// Host function: Set constant value.
///
/// # Permissions
/// Requires `WriteConstants` permission.
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `constant_name` - Name of constant
/// * `value_bytes` - Raw bytes to write
///
/// # Returns
/// ApiResponse indicating whether the write is authorized. This function
/// only checks permission — it does not write anything itself. The actual
/// write happens after the plugin's synchronous run finishes: the caller
/// (`plugin_system`'s `set_constant` host function) records a
/// `PluginProposal::SetConstant` on success here, and `execute_wasm_plugin`
/// applies it afterward via the real `update_constant` command, the same
/// path a user-driven edit takes.
pub fn api_set_constant(
    granted: &[Permission],
    _constant_name: &str,
    _value_bytes: &[u8],
) -> ApiResponse {
    if !granted.contains(&Permission::WriteConstants) {
        return ApiResponse::permission_denied("WriteConstants");
    }

    ApiResponse::ok_empty()
}

/// Host function: Subscribe to realtime channel.
///
/// # Permissions
/// Requires `SubscribeChannels` permission.
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `snapshot` - Read-only channel data captured for this execute() run
/// * `channel_name` - Name of channel (e.g., "RPM", "AFR")
///
/// # Returns
/// Success if the permission is held and the channel exists in the
/// snapshot (id assignment is the caller's responsibility — see
/// `plugin_system`'s per-instance `subscribed_channels` list).
pub fn api_subscribe_channel(
    granted: &[Permission],
    snapshot: &PluginDataSnapshot,
    channel_name: &str,
) -> ApiResponse {
    if !granted.contains(&Permission::SubscribeChannels) {
        return ApiResponse::permission_denied("SubscribeChannels");
    }
    if snapshot.channels.contains_key(channel_name) {
        ApiResponse::ok_empty()
    } else {
        ApiResponse::error(format!("channel '{}' not found", channel_name))
    }
}

/// Host function: Get realtime value for a subscribed channel.
///
/// # Permissions
/// Requires `SubscribeChannels` permission.
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `snapshot` - Read-only channel data captured for this execute() run
/// * `channel_name` - Name resolved by the caller from the id passed to
///   `subscribe_channel`
///
/// # Returns
/// ApiResponse with the channel's current value as little-endian f32 bytes,
/// or an error if the permission is missing or the channel has no value in
/// this snapshot (e.g. not connected to an ECU).
pub fn api_get_channel_value(
    granted: &[Permission],
    snapshot: &PluginDataSnapshot,
    channel_name: &str,
) -> ApiResponse {
    if !granted.contains(&Permission::SubscribeChannels) {
        return ApiResponse::permission_denied("SubscribeChannels");
    }
    match snapshot.channels.get(channel_name) {
        Some(value) => ApiResponse::ok((*value as f32).to_le_bytes().to_vec()),
        None => ApiResponse::error(format!("channel '{}' has no current value", channel_name)),
    }
}

/// Host function: Execute action sequence.
///
/// # Permissions
/// Requires `ExecuteActions` permission.
///
/// # Arguments
/// * `granted` - Permissions granted to the calling plugin instance
/// * `action_json` - JSON-encoded action data
///
/// # Returns
/// ApiResponse indicating whether the permission check passed — **not**
/// whether the action was executed. LibreTune's action-scripting engine
/// (`action_scripting::Action`/`ActionPlayer`) is validation-only today,
/// with no generic "run this one action now" dispatcher to call into, so
/// nothing here ever actually executes the action. The caller
/// (`plugin_system`'s `execute_action` host function) reflects this
/// honestly at the wire level: it returns `host_result::ACCEPTED_NOT_EXECUTED`
/// rather than `host_result::OK` on this path, so a plugin can't mistake
/// "permission granted, proposal recorded" for "actually ran."
pub fn api_execute_action(granted: &[Permission], _action_json: &str) -> ApiResponse {
    if !granted.contains(&Permission::ExecuteActions) {
        return ApiResponse::permission_denied("ExecuteActions");
    }

    ApiResponse::ok_empty()
}

/// Host function: Log a message from plugin.
///
/// No special permissions required (logging is always allowed).
///
/// # Arguments
/// * `plugin_name` - Name of calling plugin
/// * `level` - Log level (0=Debug, 1=Info, 2=Warn, 3=Error)
/// * `message` - Message text
pub fn api_log_message(
    plugin_name: impl Into<String>,
    level: i32,
    message: impl Into<String>,
) -> ApiResponse {
    let log_level = LogLevel::from_code(level);
    let log_msg = PluginLogMessage::new(plugin_name, log_level, message);

    // In real implementation, would write to log file or channel
    eprintln!("{}", log_msg.format_display());

    ApiResponse::ok_empty()
}

/// Host function: Get plugin info.
///
/// Returns plugin manifest and current stats.
///
/// # Arguments
/// * `plugin_name` - Name of plugin to query
///
/// # Returns
/// ApiResponse with JSON-encoded plugin info
pub fn api_get_plugin_info(_ctx: &PluginApiContext, plugin_name: &str) -> ApiResponse {
    if let Ok(manager) = _ctx.plugin_manager.lock() {
        if let Some(plugin) = manager.get_plugin(plugin_name) {
            let manifest = plugin.manifest();
            let stats = plugin.stats();

            let info = format!(
                r#"{{"name":"{}","version":"{}","state":"{}","exec_count":{}}}"#,
                manifest.name,
                manifest.version,
                match stats.state {
                    crate::plugin_system::PluginState::Loaded => "loaded",
                    crate::plugin_system::PluginState::Ready => "ready",
                    crate::plugin_system::PluginState::Running => "running",
                    crate::plugin_system::PluginState::Unloading => "unloading",
                    crate::plugin_system::PluginState::Disabled => "disabled",
                },
                stats.exec_count
            );

            return ApiResponse::ok(info.into_bytes());
        }
    }

    ApiResponse::error("Plugin not found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_system::{PluginConfig, PluginManager};

    fn create_test_context() -> PluginApiContext {
        let config = PluginConfig {
            data_dir: "/tmp".to_string(),
            ecu_type: "Speeduino".to_string(),
            libretune_version: "0.1.0".to_string(),
        };
        let manager = PluginManager::new(config);
        PluginApiContext::new(manager)
    }

    /// A snapshot with one table, one scalar constant, and one channel —
    /// enough to exercise both the "found" and "not found" paths.
    fn test_snapshot() -> PluginDataSnapshot {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "veTable1".to_string(),
            crate::plugin_system::TableSnapshot {
                x_bins: vec![0.0, 1.0],
                y_bins: vec![0.0],
                z_values: vec![vec![12.3, 45.6]],
            },
        );
        let mut constants = std::collections::HashMap::new();
        constants.insert("rpm".to_string(), 850.0);
        let mut channels = std::collections::HashMap::new();
        channels.insert("RPM".to_string(), 850.0);
        PluginDataSnapshot {
            tables,
            constants,
            channels,
        }
    }

    #[test]
    fn test_api_response_ok() {
        let resp = ApiResponse::ok(vec![1, 2, 3]);
        assert!(resp.success);
        assert_eq!(resp.data, vec![1, 2, 3]);
        assert!(resp.error.is_empty());
    }

    #[test]
    fn test_api_response_ok_empty() {
        let resp = ApiResponse::ok_empty();
        assert!(resp.success);
        assert!(resp.data.is_empty());
    }

    #[test]
    fn test_api_response_error() {
        let resp = ApiResponse::error("test error");
        assert!(!resp.success);
        assert_eq!(resp.error, "test error");
    }

    #[test]
    fn test_api_response_permission_denied() {
        let resp = ApiResponse::permission_denied("WriteConstants");
        assert!(!resp.success);
        assert!(resp.error.contains("WriteConstants"));
    }

    #[test]
    fn test_log_level_from_code() {
        assert_eq!(LogLevel::from_code(0), LogLevel::Debug);
        assert_eq!(LogLevel::from_code(1), LogLevel::Info);
        assert_eq!(LogLevel::from_code(2), LogLevel::Warn);
        assert_eq!(LogLevel::from_code(3), LogLevel::Error);
        assert_eq!(LogLevel::from_code(99), LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_plugin_log_message_creation() {
        let msg = PluginLogMessage::new("test_plugin", LogLevel::Info, "test message");
        assert_eq!(msg.plugin_name, "test_plugin");
        assert_eq!(msg.level, LogLevel::Info);
        assert_eq!(msg.message, "test message");
        assert!(msg.timestamp_ms > 0);
    }

    #[test]
    fn test_plugin_log_message_format() {
        let msg = PluginLogMessage::new("test", LogLevel::Error, "failure");
        let formatted = msg.format_display();
        assert!(formatted.contains("test"));
        assert!(formatted.contains("ERROR"));
        assert!(formatted.contains("failure"));
    }

    #[test]
    fn test_plugin_api_context_creation() {
        let ctx = create_test_context();
        assert_eq!(ctx.plugin_manager.lock().unwrap().count(), 0);
    }

    #[test]
    fn test_api_get_table_data_no_permission() {
        let resp = api_get_table_data(&[], &test_snapshot(), "veTable1", 0, 0);
        assert!(!resp.success);
        assert!(resp.is_permission_denied());
    }

    #[test]
    fn test_api_get_table_data_with_permission() {
        let resp = api_get_table_data(
            &[Permission::ReadTables],
            &test_snapshot(),
            "veTable1",
            0,
            0,
        );
        assert!(resp.success);
        assert_eq!(resp.data.len(), 4);
        assert_eq!(f32::from_le_bytes(resp.data.try_into().unwrap()), 12.3f32);
    }

    #[test]
    fn test_api_get_table_data_unknown_table() {
        // Permission granted, but the table isn't in the snapshot — a
        // distinct failure from permission denial.
        let resp = api_get_table_data(&[Permission::ReadTables], &test_snapshot(), "nope", 0, 0);
        assert!(!resp.success);
        assert!(!resp.is_permission_denied());
    }

    #[test]
    fn test_api_get_table_data_cell_out_of_range() {
        let resp = api_get_table_data(
            &[Permission::ReadTables],
            &test_snapshot(),
            "veTable1",
            99,
            99,
        );
        assert!(!resp.success);
        assert!(!resp.is_permission_denied());
    }

    #[test]
    fn test_api_get_constant_no_permission() {
        let resp = api_get_constant(&[], &test_snapshot(), "rpm");
        assert!(!resp.success);
    }

    #[test]
    fn test_api_get_constant_with_permission() {
        let resp = api_get_constant(&[Permission::ReadTables], &test_snapshot(), "rpm");
        assert!(resp.success);
        assert_eq!(f32::from_le_bytes(resp.data.try_into().unwrap()), 850.0f32);
    }

    #[test]
    fn test_api_get_constant_unknown() {
        let resp = api_get_constant(&[Permission::ReadTables], &test_snapshot(), "nope");
        assert!(!resp.success);
        assert!(!resp.is_permission_denied());
    }

    #[test]
    fn test_api_set_constant_no_permission() {
        let resp = api_set_constant(&[], "rpm", &[0, 0, 0, 0]);
        assert!(!resp.success);
    }

    #[test]
    fn test_api_set_constant_with_permission() {
        let resp = api_set_constant(&[Permission::WriteConstants], "rpm", &[0, 0, 0, 0]);
        assert!(resp.success);
    }

    #[test]
    fn test_api_set_constant_read_permission_insufficient() {
        // ReadTables alone must not satisfy a WriteConstants check.
        let resp = api_set_constant(&[Permission::ReadTables], "rpm", &[0, 0, 0, 0]);
        assert!(!resp.success);
    }

    #[test]
    fn test_api_subscribe_channel_no_permission() {
        let resp = api_subscribe_channel(&[], &test_snapshot(), "RPM");
        assert!(!resp.success);
    }

    #[test]
    fn test_api_subscribe_channel_with_permission() {
        let resp = api_subscribe_channel(&[Permission::SubscribeChannels], &test_snapshot(), "RPM");
        assert!(resp.success);
    }

    #[test]
    fn test_api_subscribe_channel_unknown() {
        let resp =
            api_subscribe_channel(&[Permission::SubscribeChannels], &test_snapshot(), "nope");
        assert!(!resp.success);
        assert!(!resp.is_permission_denied());
    }

    #[test]
    fn test_api_get_channel_value_no_permission() {
        let resp = api_get_channel_value(&[], &test_snapshot(), "RPM");
        assert!(!resp.success);
    }

    #[test]
    fn test_api_get_channel_value_with_permission() {
        let resp = api_get_channel_value(&[Permission::SubscribeChannels], &test_snapshot(), "RPM");
        assert!(resp.success);
        assert_eq!(f32::from_le_bytes(resp.data.try_into().unwrap()), 850.0f32);
    }

    #[test]
    fn test_api_execute_action_no_permission() {
        let resp = api_execute_action(&[], "{}");
        assert!(!resp.success);
    }

    #[test]
    fn test_api_execute_action_with_permission() {
        let resp = api_execute_action(&[Permission::ExecuteActions], "{}");
        assert!(resp.success);
    }

    #[test]
    fn test_api_log_message_always_allowed() {
        let resp = api_log_message("test_plugin", 1, "test log");
        assert!(resp.success);
    }

    #[test]
    fn test_api_get_plugin_info_not_found() {
        let ctx = create_test_context();
        let resp = api_get_plugin_info(&ctx, "nonexistent");
        assert!(!resp.success);
    }

    #[test]
    fn test_api_response_data_integrity() {
        let data = vec![1, 2, 3, 4, 5];
        let resp = ApiResponse::ok(data.clone());
        assert_eq!(resp.data, data);
    }

    #[test]
    fn test_log_level_ordering() {
        // Verify numeric ordering matches severity
        assert!((LogLevel::Debug as i32) < (LogLevel::Info as i32));
        assert!((LogLevel::Info as i32) < (LogLevel::Warn as i32));
        assert!((LogLevel::Warn as i32) < (LogLevel::Error as i32));
    }
}
