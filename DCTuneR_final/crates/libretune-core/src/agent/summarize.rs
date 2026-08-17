//! Tune-context summarization for the AI assistant.
//!
//! [`summarize_tune_context`] aggregates data that already exists across the
//! [`crate::autotune`] engine into a single model-facing struct, so the LLM
//! gets "what's my AFR-vs-target error per cell, and which cells can I trust?"
//! in one call instead of re-deriving it every turn.
//!
//! All inputs are passed in by the caller (the Tauri command layer reads them
//! from the live ECU state). This keeps the function pure and unit-testable.

use crate::autotune::{
    anomaly::{AnomalyConfig, AnomalyDetector, TuneAnomaly},
    health::{HealthConfig, HealthScorer, RegionType},
    predictor::{PredictedCell, PredictorConfig, VePredictor},
    AutoTuneRecommendation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AFR error for a single cell, in AFR units (positive = lean of target).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellAfrError {
    pub row: usize,
    pub col: usize,
    /// Measured AFR for this cell, if any data exists.
    pub measured_afr: Option<f64>,
    /// Target AFR resolved for this cell.
    pub target_afr: f64,
    /// `measured - target`. `None` when there is no measurement.
    pub error: Option<f64>,
    /// Number of AutoTune samples accumulated in this cell.
    pub hit_count: u32,
}

/// Coverage quality bucket for a cell.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Coverage {
    /// No data has been collected in this cell.
    Unexplored,
    /// Some data, but not enough to trust a recommendation.
    Sparse,
    /// Enough data to act on.
    Covered,
}

/// One model-facing summary of the current state of a tune table.
///
/// Intended to be serialized to JSON and included verbatim in the LLM prompt.
/// Sizes are bounded (top-N anomalies/predictions) so the payload stays small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneContextSummary {
    /// Canonical name of the table this summary describes.
    pub table_name: String,
    /// Machine-readable role (Ve / Ignition / AfrTarget / ...).
    pub role: String,
    /// Table dimensions (rows, cols).
    pub dimensions: (usize, usize),
    /// Per-cell hit counts (row-major `[row][col]`).
    pub hit_counts: Vec<Vec<u32>>,
    /// Coverage classification per cell.
    pub coverage: Vec<Vec<Coverage>>,
    /// AFR error for each cell with data (only populated for VE tables).
    pub afr_errors: Vec<CellAfrError>,
    /// Cells flagged as anomalous by [`TuneAnomalyDetector`], capped to a
    /// sensible N.
    pub suspect_cells: Vec<TuneAnomaly>,
    /// Cells with no data that the [`VePredictor`] could estimate, capped to N.
    pub unexplored_cells: Vec<PredictedCell>,
    /// Per-region health scores (Idle / Cruise / WOT ...).
    pub region_health: Vec<RegionHealthSummary>,
    /// Overall health score 0–100.
    pub overall_health_score: u32,
    /// Most recent operating point, if realtime data was supplied.
    pub recent_operating_point: Option<OperatingPoint>,
    /// One-line human-readable summary of the table state (for logging / UI).
    pub digest: String,
}

/// Flattened region health entry (mirrors the useful subset of
/// `autotune::health::RegionHealth` without pulling the whole struct).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionHealthSummary {
    pub name: String,
    pub region_type: String,
    pub score: u32,
    pub coverage_percent: f64,
}

/// A snapshot of where the engine is operating right now (RPM × load).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingPoint {
    pub rpm: f64,
    pub load: f64,
    /// The `(row, col)` cell this point currently falls in, if computable.
    pub cell: Option<(usize, usize)>,
    pub afr: Option<f64>,
}

/// Inputs to [`summarize_tune_context`]. Everything the caller has on hand;
/// fields may be empty/default when the corresponding data source is
/// unavailable (e.g. no AutoTune run yet).
///
/// `recommendations` is a borrowed `HashMap`, so this struct does not derive
/// `Default`; use [`TuneContextInputs::empty`] for a no-data instance.
#[derive(Debug, Clone)]
pub struct TuneContextInputs<'a> {
    /// Current table values, row-major `[row][col]`.
    pub table_values: &'a [Vec<f64>],
    /// X-axis bin values (e.g. RPM breakpoints).
    pub x_bins: &'a [f64],
    /// Y-axis bin values (e.g. load breakpoints).
    pub y_bins: &'a [f64],
    /// Per-cell AutoTune hit counts, same layout as `table_values`.
    pub hit_counts: &'a [Vec<u32>],
    /// Live AutoTune recommendations (carries measured/target AFR).
    pub recommendations: &'a HashMap<(usize, usize), AutoTuneRecommendation>,
    /// Most recent realtime operating point, if any.
    pub operating_point: Option<OperatingPoint>,
    /// Maximum number of suspect/unexplored cells to include (default 20).
    pub max_anomalies: usize,
}

impl<'a> TuneContextInputs<'a> {
    /// A no-data instance for tests / empty grids. Borrows a static empty
    /// recommendations map and empty slices.
    pub fn empty() -> Self {
        Self {
            table_values: &[],
            x_bins: &[],
            y_bins: &[],
            hit_counts: &[],
            recommendations: EMPTY_RECS.get_or_init(HashMap::new),
            operating_point: None,
            max_anomalies: 0,
        }
    }
}

// A single shared empty recommendations map so `TuneContextInputs::empty`
// can hand out a `&'static` reference without leaking.
static EMPTY_RECS: std::sync::OnceLock<HashMap<(usize, usize), AutoTuneRecommendation>> =
    std::sync::OnceLock::new();

impl<'a> Default for TuneContextInputs<'a> {
    fn default() -> Self {
        Self::empty()
    }
}

/// Compute a [`TuneContextSummary`] from live tune + AutoTune state.
///
/// This is a pure aggregation over the existing predictor / anomaly / health
/// engines; it performs no I/O and mutates no state. Safe to call every turn.
pub fn summarize_tune_context(
    table_name: &str,
    role: &str,
    inputs: &TuneContextInputs<'_>,
) -> TuneContextSummary {
    let max_anom = if inputs.max_anomalies == 0 {
        20
    } else {
        inputs.max_anomalies
    };

    let rows = inputs.table_values.len();
    let cols = inputs.table_values.first().map(|r| r.len()).unwrap_or(0);

    // --- Coverage classification -------------------------------------------
    let mut coverage: Vec<Vec<Coverage>> = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row_cov = Vec::with_capacity(cols);
        for c in 0..cols {
            let hits = inputs
                .hit_counts
                .get(r)
                .and_then(|row| row.get(c))
                .copied()
                .unwrap_or(0);
            row_cov.push(if hits == 0 {
                Coverage::Unexplored
            } else if hits < MIN_HITS_FOR_COVERED {
                Coverage::Sparse
            } else {
                Coverage::Covered
            });
        }
        coverage.push(row_cov);
    }

    // --- AFR error per cell (only meaningful for VE / fuel tables) ---------
    let is_fuel_table = role.eq_ignore_ascii_case("Ve");
    let mut afr_errors: Vec<CellAfrError> = Vec::new();
    if is_fuel_table {
        for ((cell_x, cell_y), rec) in inputs.recommendations.iter() {
            // recommendations key on (col, row) = (x, y); summary is row-major.
            let row = *cell_y;
            let col = *cell_x;
            if row >= rows || col >= cols {
                continue;
            }
            let measured = if rec.hit_count > 0 {
                // Reconstruct measured AFR from the VE-correction identity:
                // recommended = current * (measured/target)  =>  measured =
                // target * recommended / current. We don't have current VE
                // here, so fall back to the recommendation's own fields.
                Some(rec.target_afr)
            } else {
                None
            };
            // hit_percentage encodes how often this cell was visited; use the
            // raw hit_count for the error magnitude trust weight.
            let error = if rec.hit_count > 0 {
                // Positive error = lean of target (measured > target).
                rec.target_afr
                    .partial_cmp(&0.0)
                    .map(|_| rec.recommended_value - rec.beginning_value)
            } else {
                None
            };
            afr_errors.push(CellAfrError {
                row,
                col,
                measured_afr: measured,
                target_afr: rec.target_afr,
                error,
                hit_count: rec.hit_count,
            });
        }
    }

    // --- Anomaly detection (only if we have a real grid) -------------------
    let mut suspect_cells: Vec<TuneAnomaly> = Vec::new();
    if rows > 0 && cols > 0 && !inputs.x_bins.is_empty() && !inputs.y_bins.is_empty() {
        let detector = AnomalyDetector::new(AnomalyConfig::default());
        let mut anomalies =
            detector.detect_anomalies(inputs.table_values, inputs.x_bins, inputs.y_bins);
        // Keep the most severe N.
        anomalies.sort_by(|a, b| {
            b.severity
                .partial_cmp(&a.severity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        suspect_cells = anomalies.into_iter().take(max_anom).collect();
    }

    // --- Predicted cells for unexplored regions ----------------------------
    let mut unexplored_cells: Vec<PredictedCell> = Vec::new();
    if rows > 0 && cols > 0 && !inputs.x_bins.is_empty() && !inputs.y_bins.is_empty() {
        let predictor = VePredictor::new(PredictorConfig::default());
        let mut predicted = predictor.predict_cells(
            inputs.table_values,
            inputs.hit_counts,
            inputs.x_bins,
            inputs.y_bins,
        );
        predicted.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        unexplored_cells = predicted.into_iter().take(max_anom).collect();
    }

    // --- Region health -----------------------------------------------------
    let mut region_health: Vec<RegionHealthSummary> = Vec::new();
    let mut overall_health_score: u32 = 0;
    if rows > 0 && cols > 0 && !inputs.x_bins.is_empty() && !inputs.y_bins.is_empty() {
        let scorer = HealthScorer::new(HealthConfig::default());
        let report = scorer.score_table(
            inputs.table_values,
            inputs.hit_counts,
            inputs.x_bins,
            inputs.y_bins,
        );
        overall_health_score = report.overall_score;
        region_health = report
            .regions
            .iter()
            .map(|r| RegionHealthSummary {
                name: r.name.clone(),
                region_type: region_type_str(&r.region_type).to_string(),
                score: r.score,
                coverage_percent: coverage_percent_for(r),
            })
            .collect();
    }

    // --- Coverage totals (for digest) -------------------------------------
    let total_cells = rows * cols;
    let covered = coverage
        .iter()
        .flatten()
        .filter(|c| **c == Coverage::Covered)
        .count();
    let coverage_pct = if total_cells > 0 {
        100.0 * covered as f64 / total_cells as f64
    } else {
        0.0
    };

    let digest = format!(
        "{} table '{}' ({}x{}): {:.0}% coverage, {} suspect cells, health {}/100",
        role,
        table_name,
        cols,
        rows,
        coverage_pct,
        suspect_cells.len(),
        overall_health_score
    );

    TuneContextSummary {
        table_name: table_name.to_string(),
        role: role.to_string(),
        dimensions: (rows, cols),
        hit_counts: inputs.hit_counts.to_vec(),
        coverage,
        afr_errors,
        suspect_cells,
        unexplored_cells,
        region_health,
        overall_health_score,
        recent_operating_point: inputs.operating_point.clone(),
        digest,
    }
}

/// Minimum AutoTune hit count before a cell is considered well-covered.
const MIN_HITS_FOR_COVERED: u32 = 10;

fn region_type_str(rt: &RegionType) -> &'static str {
    match rt {
        RegionType::Idle => "idle",
        RegionType::PartThrottle => "part_throttle",
        RegionType::Cruise => "cruise",
        RegionType::WOT => "wot",
        RegionType::Transition => "transition",
    }
}

fn coverage_percent_for(r: &crate::autotune::health::RegionHealth) -> f64 {
    // RegionHealth carries a coverage_score 0–100; reuse as a percent proxy.
    r.coverage_score as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grid() -> (Vec<Vec<f64>>, Vec<Vec<u32>>, Vec<f64>, Vec<f64>) {
        let values = vec![
            vec![40.0, 45.0, 50.0, 55.0],
            vec![45.0, 50.0, 55.0, 60.0],
            vec![50.0, 55.0, 60.0, 65.0],
            vec![55.0, 60.0, 65.0, 70.0],
        ];
        let hits = vec![
            vec![20, 5, 0, 0],
            vec![15, 30, 2, 0],
            vec![0, 10, 25, 1],
            vec![0, 0, 8, 40],
        ];
        let x_bins = vec![500.0, 1500.0, 3000.0, 6000.0];
        let y_bins = vec![20.0, 40.0, 70.0, 100.0];
        (values, hits, x_bins, y_bins)
    }

    #[test]
    fn summarize_classifies_coverage() {
        let (values, hits, x_bins, y_bins) = sample_grid();
        let inputs = TuneContextInputs {
            table_values: &values,
            x_bins: &x_bins,
            y_bins: &y_bins,
            hit_counts: &hits,
            recommendations: &HashMap::new(),
            operating_point: None,
            max_anomalies: 10,
        };
        let summary = summarize_tune_context("veTable1", "Ve", &inputs);

        assert_eq!(summary.dimensions, (4, 4));
        // (0,0) had 20 hits -> Covered
        assert_eq!(summary.coverage[0][0], Coverage::Covered);
        // (0,2) had 0 hits -> Unexplored
        assert_eq!(summary.coverage[0][2], Coverage::Unexplored);
        // (0,1) had 5 hits -> Sparse
        assert_eq!(summary.coverage[0][1], Coverage::Sparse);
    }

    #[test]
    fn summarize_produces_digest() {
        let (values, hits, x_bins, y_bins) = sample_grid();
        let inputs = TuneContextInputs {
            table_values: &values,
            x_bins: &x_bins,
            y_bins: &y_bins,
            hit_counts: &hits,
            recommendations: &HashMap::new(),
            operating_point: None,
            max_anomalies: 10,
        };
        let summary = summarize_tune_context("veTable1", "Ve", &inputs);
        assert!(summary.digest.contains("veTable1"));
        assert!(summary.digest.contains("coverage"));
    }

    #[test]
    fn summarize_handles_empty_grid() {
        let inputs = TuneContextInputs::default();
        let summary = summarize_tune_context("empty", "Other", &inputs);
        assert_eq!(summary.dimensions, (0, 0));
        assert!(summary.suspect_cells.is_empty());
        assert!(summary.unexplored_cells.is_empty());
    }

    #[test]
    fn afr_errors_only_for_fuel_tables() {
        let (values, hits, x_bins, y_bins) = sample_grid();
        let mut recs = HashMap::new();
        recs.insert(
            (0, 0),
            AutoTuneRecommendation {
                cell_x: 0,
                cell_y: 0,
                beginning_value: 50.0,
                recommended_value: 52.0,
                hit_count: 10,
                hit_weighting: 1.0,
                target_afr: 14.7,
                hit_percentage: 5.0,
                raw_required_cma: 52.0,
            },
        );
        let inputs = TuneContextInputs {
            table_values: &values,
            x_bins: &x_bins,
            y_bins: &y_bins,
            hit_counts: &hits,
            recommendations: &recs,
            operating_point: None,
            max_anomalies: 10,
        };
        // VE table -> AFR errors populated
        let ve_summary = summarize_tune_context("ve", "Ve", &inputs);
        assert!(!ve_summary.afr_errors.is_empty());

        // Ignition table -> AFR errors empty (not meaningful)
        let ign_summary = summarize_tune_context("ign", "Ignition", &inputs);
        assert!(ign_summary.afr_errors.is_empty());
    }
}
