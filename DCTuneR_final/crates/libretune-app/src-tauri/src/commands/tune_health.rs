//! Tune anomaly detection, health scoring, predicted-fill, and dyno overlay commands.

use crate::get_table_data_internal;
use crate::state::AppState;

/// Get predicted VE table fill values for low-coverage cells.
#[tauri::command]
pub async fn get_predicted_fills(
    state: tauri::State<'_, AppState>,
    table_name: String,
    min_confidence: Option<f64>,
    min_hit_count: Option<u32>,
) -> Result<Vec<libretune_core::autotune::predictor::PredictedCell>, String> {
    use libretune_core::autotune::predictor::{PredictorConfig, VePredictor};

    let table_data = get_table_data_internal(&state, &table_name).await?;

    let table_values = &table_data.z_values;
    let rows = table_values.len();
    let cols = if rows > 0 { table_values[0].len() } else { 0 };

    let at_guard = state.autotune_state.lock().await;
    let recs = at_guard.get_recommendations();
    let mut hit_counts = vec![vec![0u32; cols]; rows];
    for rec in &recs {
        if rec.cell_y < rows && rec.cell_x < cols {
            hit_counts[rec.cell_y][rec.cell_x] = rec.hit_count;
        }
    }
    drop(at_guard);

    let config = PredictorConfig {
        min_confidence: min_confidence.unwrap_or(0.3),
        min_hit_count: min_hit_count.unwrap_or(3),
        ..Default::default()
    };

    let predictor = VePredictor::new(config);

    Ok(predictor.predict_cells(
        table_values,
        &hit_counts,
        &table_data.x_bins,
        &table_data.y_bins,
    ))
}

/// Detect anomalies in a VE/fuel table.
#[tauri::command]
pub async fn get_tune_anomalies(
    state: tauri::State<'_, AppState>,
    table_name: String,
    outlier_sigma: Option<f64>,
) -> Result<Vec<libretune_core::autotune::anomaly::TuneAnomaly>, String> {
    use libretune_core::autotune::anomaly::{AnomalyConfig, AnomalyDetector};

    let table_data = get_table_data_internal(&state, &table_name).await?;

    let config = AnomalyConfig {
        outlier_sigma: outlier_sigma.unwrap_or(2.0),
        ..Default::default()
    };

    let detector = AnomalyDetector::new(config);

    Ok(detector.detect_anomalies(&table_data.z_values, &table_data.x_bins, &table_data.y_bins))
}

/// Get a tune health report scoring the VE table by region.
#[tauri::command]
pub async fn get_tune_health_report(
    state: tauri::State<'_, AppState>,
    table_name: String,
) -> Result<libretune_core::autotune::health::TuneHealthReport, String> {
    use libretune_core::autotune::health::{HealthConfig, HealthScorer};

    let table_data = get_table_data_internal(&state, &table_name).await?;

    let table_values = &table_data.z_values;
    let rows = table_values.len();
    let cols = if rows > 0 { table_values[0].len() } else { 0 };

    let at_guard = state.autotune_state.lock().await;
    let recs = at_guard.get_recommendations();
    let mut hit_counts = vec![vec![0u32; cols]; rows];
    for rec in &recs {
        if rec.cell_y < rows && rec.cell_x < cols {
            hit_counts[rec.cell_y][rec.cell_x] = rec.hit_count;
        }
    }
    drop(at_guard);

    let scorer = HealthScorer::new(HealthConfig::default());

    Ok(scorer.score_table(
        table_values,
        &hit_counts,
        &table_data.x_bins,
        &table_data.y_bins,
    ))
}

/// Map dyno data onto a table for overlay visualization.
#[tauri::command]
pub async fn get_dyno_table_overlay(
    state: tauri::State<'_, AppState>,
    table_name: String,
    dyno_run: libretune_core::datalog::dyno::DynoRun,
) -> Result<libretune_core::datalog::dyno::DynoTableOverlay, String> {
    let table_data = get_table_data_internal(&state, &table_name).await?;
    Ok(dyno_run.map_to_table(&table_data.x_bins, &table_data.y_bins, None))
}
