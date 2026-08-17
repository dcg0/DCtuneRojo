//! Plugin API integration tests
//!
//! Tests the WASM host function API surface and permission enforcement.
//!
//! # Test fixture invariant
//!
//! Permission-guarded host functions take the calling plugin's *granted*
//! permission set directly (`&[Permission]`), not a shared [`PluginManager`]
//! lookup — see [`libretune_core::plugin_system`]'s host-function wiring for
//! why (avoids re-entrant locking from inside a WASM call). Passing `&[]`
//! below is the "unregistered/no-grant" case; passing the specific
//! `Permission` variant is the "granted" case.

use libretune_core::plugin_api::*;
use libretune_core::plugin_system::*;

/// Build an API context wrapping an empty [`PluginManager`].
///
/// Only used by [`api_get_plugin_info`], which still looks a plugin up by
/// name in the manager (it reports on a *different* plugin's info, not the
/// caller's own permissions, so it legitimately needs the manager).
fn create_test_context() -> PluginApiContext {
    let config = PluginConfig {
        data_dir: "/tmp/libretune_plugins".to_string(),
        ecu_type: "Speeduino".to_string(),
        libretune_version: "0.1.0".to_string(),
    };
    let manager = PluginManager::new(config);
    PluginApiContext::new(manager)
}

/// A snapshot with one table, one scalar constant, and one channel, for
/// tests that need to exercise the "permission granted and data present"
/// success path rather than just the permission gate.
fn test_snapshot() -> PluginDataSnapshot {
    let mut tables = std::collections::HashMap::new();
    tables.insert(
        "veTable1".to_string(),
        TableSnapshot {
            x_bins: vec![0.0, 1.0],
            y_bins: vec![0.0],
            z_values: vec![vec![12.3, 45.6]],
        },
    );
    let mut constants = std::collections::HashMap::new();
    constants.insert("rpmMin".to_string(), 400.0);
    let mut channels = std::collections::HashMap::new();
    channels.insert("RPM".to_string(), 850.0);
    PluginDataSnapshot {
        tables,
        constants,
        channels,
    }
}

#[test]
fn test_api_response_serialization() {
    let resp = ApiResponse::ok(vec![1, 2, 3, 4]);
    assert!(resp.success);
    assert_eq!(resp.data.len(), 4);
}

#[test]
fn test_api_response_error_message() {
    let resp = ApiResponse::error("Test error");
    assert!(!resp.success);
    assert_eq!(resp.error, "Test error");
}

#[test]
fn test_permission_denied_response() {
    let resp = ApiResponse::permission_denied("ReadTables");
    assert!(!resp.success);
    assert!(resp.error.contains("ReadTables"));
    assert!(resp.error.contains("Permission denied"));
}

#[test]
fn test_api_context_initialization() {
    let ctx = create_test_context();
    assert_eq!(ctx.plugin_manager.lock().unwrap().count(), 0);
}

#[test]
fn test_api_get_table_data_permission_check() {
    // No permissions granted: must be rejected before any table data is read.
    let resp = api_get_table_data(&[], &test_snapshot(), "veTable1", 0, 0);
    assert!(!resp.success);
    assert!(resp.error.contains("Permission"));
}

#[test]
fn test_api_get_table_data_granted() {
    let resp = api_get_table_data(
        &[Permission::ReadTables],
        &test_snapshot(),
        "veTable1",
        0,
        0,
    );
    assert!(resp.success);
}

#[test]
fn test_api_get_constant_permission_check() {
    // Constants are covered by ReadTables (see api_get_constant docs); no
    // grant means the call must be rejected.
    let resp = api_get_constant(&[], &test_snapshot(), "rpmMin");
    assert!(!resp.success);
}

#[test]
fn test_api_set_constant_permission_check() {
    // Writes are a distinct permission from reads: ReadTables alone must not
    // satisfy WriteConstants, so the byte payload must never reach the tune cache.
    let resp = api_set_constant(&[Permission::ReadTables], "rpmMin", &[0, 0, 0, 0]);
    assert!(!resp.success);
    assert!(resp.error.contains("WriteConstants"));
}

#[test]
fn test_api_set_constant_granted() {
    let resp = api_set_constant(&[Permission::WriteConstants], "rpmMin", &[0, 0, 0, 0]);
    assert!(resp.success);
}

#[test]
fn test_api_subscribe_channel_permission_check() {
    // No SubscribeChannels grant: cannot register a realtime subscription and
    // must be rejected up front.
    let resp = api_subscribe_channel(&[], &test_snapshot(), "RPM");
    assert!(!resp.success);
}

#[test]
fn test_api_get_channel_value_permission_check() {
    // Reading a subscribed channel value re-checks SubscribeChannels, so even
    // a channel that exists in the snapshot must be rejected without the grant.
    let resp = api_get_channel_value(&[], &test_snapshot(), "RPM");
    assert!(!resp.success);
}

#[test]
fn test_api_execute_action_permission_check() {
    // No ExecuteActions grant: the JSON action payload must be rejected
    // before any command is parsed or dispatched.
    let resp = api_execute_action(&[], "{}");
    assert!(!resp.success);
    assert!(resp.error.contains("ExecuteActions"));
}

#[test]
fn test_api_log_message_always_allowed() {
    // Logging deliberately bypasses the permission gate — it is the one host
    // function any plugin may call regardless of grants.
    let resp = api_log_message("test_plugin", 1, "Info message");
    assert!(resp.success);

    let resp = api_log_message("test_plugin", 3, "Error message");
    assert!(resp.success);

    let resp = api_log_message("another", 0, "Debug");
    assert!(resp.success);
}

#[test]
fn test_log_level_all_variants() {
    let levels = vec![
        (0, LogLevel::Debug),
        (1, LogLevel::Info),
        (2, LogLevel::Warn),
        (3, LogLevel::Error),
        (99, LogLevel::Error), // Out of range defaults to Error
    ];

    for (code, expected) in levels {
        assert_eq!(LogLevel::from_code(code), expected);
    }
}

#[test]
fn test_log_level_display_strings() {
    assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
    assert_eq!(LogLevel::Info.as_str(), "INFO");
    assert_eq!(LogLevel::Warn.as_str(), "WARN");
    assert_eq!(LogLevel::Error.as_str(), "ERROR");
}

#[test]
fn test_plugin_log_message_timestamp() {
    let msg = PluginLogMessage::new("plugin1", LogLevel::Info, "message");
    assert!(msg.timestamp_ms > 0);

    let msg2 = PluginLogMessage::new("plugin2", LogLevel::Error, "error");
    assert!(msg2.timestamp_ms > 0);
    assert!(msg2.timestamp_ms >= msg.timestamp_ms); // Later message should have >= timestamp
}

#[test]
fn test_plugin_log_message_formatting() {
    let msg = PluginLogMessage::new("my_plugin", LogLevel::Warn, "This is a warning");
    let formatted = msg.format_display();

    assert!(formatted.contains("my_plugin"));
    assert!(formatted.contains("WARN"));
    assert!(formatted.contains("This is a warning"));
}

#[test]
fn test_api_get_plugin_info_not_found() {
    let ctx = create_test_context();

    // get_plugin_info is not permission-gated, but an unknown name yields an
    // explicit "not found" error rather than permission_denied.
    let resp = api_get_plugin_info(&ctx, "nonexistent_plugin");
    assert!(!resp.success);
    assert!(resp.error.contains("not found"));
}

#[test]
fn test_api_response_empty_data() {
    let resp = ApiResponse::ok_empty();
    assert!(resp.success);
    assert!(resp.data.is_empty());
    assert!(resp.error.is_empty());
}

#[test]
fn test_api_response_large_data() {
    let large_data = vec![0u8; 65536]; // 64KB
    let resp = ApiResponse::ok(large_data.clone());
    assert!(resp.success);
    assert_eq!(resp.data.len(), 65536);
}

#[test]
fn test_plugin_log_message_different_levels() {
    let debug = PluginLogMessage::new("p", LogLevel::Debug, "debug");
    let info = PluginLogMessage::new("p", LogLevel::Info, "info");
    let warn = PluginLogMessage::new("p", LogLevel::Warn, "warn");
    let error = PluginLogMessage::new("p", LogLevel::Error, "error");

    assert_eq!(debug.level, LogLevel::Debug);
    assert_eq!(info.level, LogLevel::Info);
    assert_eq!(warn.level, LogLevel::Warn);
    assert_eq!(error.level, LogLevel::Error);
}

#[test]
fn test_api_response_error_only() {
    let resp = ApiResponse::error("Critical failure");
    assert!(!resp.success);
    assert!(resp.data.is_empty());
    assert_eq!(resp.error, "Critical failure");
}

#[test]
fn test_multiple_log_messages() {
    // Each call is independent and permission-free, so a rapid burst of logs
    // must all succeed (no rate limit or dedup at this layer).
    for i in 0..5 {
        let msg = format!("Log message {}", i);
        let resp = api_log_message("test", 1, msg);
        assert!(resp.success);
    }
}

#[test]
fn test_api_permission_enforcement_consistency() {
    // Denial must be uniform across calls: an empty grant set is rejected
    // regardless of which table/cell it targets, confirming the guard is
    // keyed on the permission set and not on the specific request.
    let resp1 = api_get_table_data(&[], &test_snapshot(), "table1", 0, 0);
    let resp2 = api_get_table_data(&[], &test_snapshot(), "table2", 5, 5);

    assert!(!resp1.success);
    assert!(!resp2.success);
    assert!(resp1.error.contains("Permission"));
    assert!(resp2.error.contains("Permission"));
}

#[test]
fn test_log_message_multiple_plugins() {
    // Logging state is never shared between plugin names, so three distinct
    // plugins can each emit a log without interference.
    let resp1 = api_log_message("plugin_a", 0, "From A");
    let resp2 = api_log_message("plugin_b", 1, "From B");
    let resp3 = api_log_message("plugin_c", 2, "From C");

    assert!(resp1.success);
    assert!(resp2.success);
    assert!(resp3.success);
}
