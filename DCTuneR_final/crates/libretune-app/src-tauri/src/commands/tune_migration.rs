//! Tune migration & metadata Tauri commands.

use libretune_core::tune::{ConstantManifestEntry, IniMetadata, MigrationReport};

use crate::state::AppState;

/// Get the current migration report (if any) from loading a tune
#[tauri::command]
pub async fn get_migration_report(
    state: tauri::State<'_, AppState>,
) -> Result<Option<MigrationReport>, String> {
    let report = state.migration_report.lock().await;
    Ok(report.clone())
}

/// Clear the current migration report
#[tauri::command]
pub async fn clear_migration_report(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.migration_report.lock().await = None;
    Ok(())
}

/// Get INI metadata for the currently loaded tune
#[tauri::command]
pub async fn get_tune_ini_metadata(
    state: tauri::State<'_, AppState>,
) -> Result<Option<IniMetadata>, String> {
    let tune = state.current_tune.lock().await;
    Ok(tune.as_ref().and_then(|t| t.ini_metadata.clone()))
}

/// Get constant manifest for the currently loaded tune
#[tauri::command]
pub async fn get_tune_constant_manifest(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<ConstantManifestEntry>>, String> {
    let tune = state.current_tune.lock().await;
    Ok(tune.as_ref().and_then(|t| t.constant_manifest.clone()))
}
