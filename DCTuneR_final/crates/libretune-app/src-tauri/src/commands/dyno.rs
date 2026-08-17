//! Dyno (chassis dynamometer) Tauri commands.

use libretune_core::datalog::dyno::{detect_csv_headers, DynoComparison, DynoRun};

/// Load a dyno CSV file and return the parsed run data
#[tauri::command]
pub async fn load_dyno_run(path: String, name: String) -> Result<DynoRun, String> {
    DynoRun::from_csv(&path, name).map_err(|e| format!("Failed to load dyno CSV: {}", e))
}

/// Detect CSV column headers for dyno import
#[tauri::command]
pub async fn detect_dyno_headers(path: String) -> Result<Vec<String>, String> {
    detect_csv_headers(&path).map_err(|e| format!("Failed to read CSV headers: {}", e))
}

/// Compare two dyno runs
#[tauri::command]
pub async fn compare_dyno_runs(run_a: DynoRun, run_b: DynoRun) -> Result<DynoComparison, String> {
    Ok(DynoComparison::compare(run_a, run_b))
}
