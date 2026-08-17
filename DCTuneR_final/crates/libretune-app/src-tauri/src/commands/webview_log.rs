//! Bridge for surfacing webview (frontend) console output in the Rust logs.
//!
//! The session logs capture Rust `tracing` only; webview `console.error`,
//! uncaught exceptions and unhandled promise rejections are otherwise
//! invisible. That blind spot is why frontend-side failures like the Engine
//! Constants dialog silently not dispatching `update_constant` (D6) could not
//! be diagnosed from a full-debug session. The frontend forwards its console
//! errors/warnings and uncaught errors here so they land in the same log.

/// Emit a message originating from the webview into the Rust `tracing` log,
/// tagged so it is obviously frontend-sourced. `level` is the JS console level
/// ("error", "warn", "log", "info", "debug"); anything else is treated as info.
#[tauri::command]
pub fn log_webview_message(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "webview", "{message}"),
        "warn" => tracing::warn!(target: "webview", "{message}"),
        "debug" => tracing::debug!(target: "webview", "{message}"),
        // "log"/"info"/unknown → info
        _ => tracing::info!(target: "webview", "{message}"),
    }
}
