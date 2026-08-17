//! System / environment Tauri commands.
//!
//! Exposes lightweight info commands like build version and serial port
//! enumeration that don't fit any specific domain.

use libretune_core::protocol::serial::list_ports;
use serde::Serialize;

#[derive(Serialize)]
pub struct BuildInfo {
    pub version: String,
    pub build_id: String,
}

/// Get application build information (version + nightly build ID).
#[tauri::command]
pub fn get_build_info(app: tauri::AppHandle) -> BuildInfo {
    let version = app.package_info().version.to_string();
    let build_id = option_env!("LIBRETUNE_BUILD_ID")
        .unwrap_or("unknown")
        .to_string();
    BuildInfo { version, build_id }
}

/// Lists all available serial ports on the system.
///
/// Returns: Vector of serial port names (e.g., "COM3" on Windows, "/dev/ttyUSB0" on Linux)
#[tauri::command]
pub async fn get_serial_ports() -> Result<Vec<String>, String> {
    Ok(list_ports().into_iter().map(|p| p.name).collect())
}
