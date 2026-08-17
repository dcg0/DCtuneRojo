//! Adaptive timing Tauri commands.

use libretune_core::ini::AdaptiveTimingConfig;
use serde::Serialize;

use crate::state::AppState;

/// Response for adaptive timing stats
#[derive(Serialize)]
pub struct AdaptiveTimingStats {
    pub enabled: bool,
    pub avg_response_ms: Option<f64>,
    pub sample_count: usize,
    pub current_timeout_ms: Option<u64>,
}

/// Enable adaptive timing (experimental feature that dynamically adjusts communication speed)
#[tauri::command]
pub async fn enable_adaptive_timing(
    state: tauri::State<'_, AppState>,
    multiplier: Option<f32>,
    min_timeout_ms: Option<u32>,
    max_timeout_ms: Option<u32>,
) -> Result<AdaptiveTimingStats, String> {
    let mut guard = state.connection.lock().await;
    let conn = guard.as_mut().ok_or("Not connected to ECU")?;

    let config = AdaptiveTimingConfig {
        enabled: true,
        multiplier: multiplier.unwrap_or(2.5),
        min_timeout_ms: min_timeout_ms.unwrap_or(10),
        max_timeout_ms: max_timeout_ms.unwrap_or(500),
        sample_count: 20,
    };

    conn.enable_adaptive_timing(Some(config));

    Ok(AdaptiveTimingStats {
        enabled: true,
        avg_response_ms: None,
        sample_count: 0,
        current_timeout_ms: None,
    })
}

/// Disable adaptive timing (return to INI-specified timing)
#[tauri::command]
pub async fn disable_adaptive_timing(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.connection.lock().await;
    let conn = guard.as_mut().ok_or("Not connected to ECU")?;

    conn.disable_adaptive_timing();
    Ok(())
}

/// Get adaptive timing statistics
#[tauri::command]
pub async fn get_adaptive_timing_stats(
    state: tauri::State<'_, AppState>,
) -> Result<AdaptiveTimingStats, String> {
    let guard = state.connection.lock().await;
    let conn = guard.as_ref().ok_or("Not connected to ECU")?;

    let enabled = conn.is_adaptive_timing_enabled();
    let stats = conn.adaptive_timing_stats();

    Ok(AdaptiveTimingStats {
        enabled,
        avg_response_ms: stats
            .as_ref()
            .map(|(avg, _)| avg.as_micros() as f64 / 1000.0),
        sample_count: stats.as_ref().map(|(_, count)| *count).unwrap_or(0),
        current_timeout_ms: None,
    })
}
