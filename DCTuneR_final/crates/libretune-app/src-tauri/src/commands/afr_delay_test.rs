//! Automated AFR transport-delay measurement.
//!
//! Measuring how long the wideband takes to see a fuelling change needs a
//! clean, precisely-timed step at a known operating point. Doing that by hand
//! means typing a value, watching a gauge, and typing it back — awkward while
//! driving, and impossible to time accurately enough for a delay of a few
//! hundred milliseconds.
//!
//! This runs the step automatically: enrich by a set percentage, hold, restore,
//! settle, repeat. The operator holds the engine at the operating point; the
//! app only touches fuelling.
//!
//! # Safety
//!
//! This writes to a running engine, so the design is deliberately narrow:
//!
//! - **Enrichment only.** The step percentage is clamped positive. A rich
//!   excursion is harmless; a lean one at load destroys pistons. There is no
//!   parameter that can make the mixture leaner.
//! - **Bounded magnitude.** Capped at [`MAX_STEP_PERCENT`], well inside the
//!   range where an engine simply runs rich.
//! - **RAM only, never burned.** The step is written to the ECU's live memory,
//!   so it is not persisted. Cycling the key restores the stored tune even if
//!   this process dies mid-step.
//! - **Restore on every path.** The original value is written back after each
//!   step, on abort, and on error. The restore is attempted even when the run
//!   is failing, and a failure to restore is escalated loudly.
//! - **Abortable.** The operator can stop between steps, and abort triggers an
//!   immediate restore.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// Largest enrichment the test will apply. Chosen so the mixture stays in the
/// range where an engine runs rich but healthy; the useful signal is around
/// 8-10% and anything beyond this is measuring nothing new.
const MAX_STEP_PERCENT: f64 = 20.0;

/// Smallest enrichment worth applying. Below this the AFR change disappears
/// into normal cycle-to-cycle scatter and no delay can be extracted.
const MIN_STEP_PERCENT: f64 = 3.0;

/// Bounds on how long a step is held, in milliseconds.
const MIN_HOLD_MS: u64 = 500;
const MAX_HOLD_MS: u64 = 5_000;

/// Constant used as the fuel multiplier. `reqFuel` scales every injector pulse
/// equally, so the step is independent of position in the VE table — unlike
/// editing a table cell, where interpolation between cells would blur it.
const FUEL_CONSTANT: &str = "reqFuel";

/// Progress event emitted to the frontend during a delay-test run (as
/// `afr_delay_test:progress`). `phase` is a coarse stage label —
/// "starting", "enriching", "settling", then "complete" or "aborted".
/// `applied_value` and `baseline_value` are the current and restore
/// fuel-constant values so the UI can show exactly what is written and
/// confirm the restore.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DelayTestProgress {
    pub phase: String,
    pub step: u32,
    pub total_steps: u32,
    pub applied_value: f64,
    pub baseline_value: f64,
    pub message: String,
}

fn emit(app: &AppHandle, p: DelayTestProgress) {
    let _ = app.emit("afr_delay_test:progress", p);
}

/// Shared abort flag so [`abort_afr_delay_test`] can stop a run in progress.
static ABORT: std::sync::OnceLock<Arc<AtomicBool>> = std::sync::OnceLock::new();

fn abort_flag() -> Arc<AtomicBool> {
    ABORT
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

/// Request that a running test stop. The current step is restored before the
/// run ends.
#[tauri::command]
pub async fn abort_afr_delay_test() -> Result<(), String> {
    abort_flag().store(true, Ordering::SeqCst);
    Ok(())
}

/// Run an automated series of enrichment steps.
///
/// `step_percent` is clamped to [`MIN_STEP_PERCENT`]..=[`MAX_STEP_PERCENT`] and
/// forced positive. `hold_ms` is how long the enrichment is applied;
/// `settle_ms` is the pause afterwards for the mixture to return to baseline
/// before the next step.
#[tauri::command]
pub async fn run_afr_delay_test(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    step_percent: f64,
    hold_ms: u64,
    settle_ms: u64,
    repeats: u32,
) -> Result<String, String> {
    // Enrichment only: take the magnitude, then clamp. A negative input cannot
    // survive this, so no combination of arguments leans the engine out.
    let step_percent = step_percent.abs().clamp(MIN_STEP_PERCENT, MAX_STEP_PERCENT);
    let hold_ms = hold_ms.clamp(MIN_HOLD_MS, MAX_HOLD_MS);
    let settle_ms = settle_ms.clamp(MIN_HOLD_MS, MAX_HOLD_MS * 2);
    let repeats = repeats.clamp(1, 20);

    // Read the baseline through the same path the UI uses, so the restore
    // target is what the engine is actually running rather than an assumption.
    // Reading it up front also fails the run early if the ECU is unreachable,
    // instead of discovering that after a step has already been applied.
    let baseline = crate::commands::constants_read::get_constant_value(
        state.clone(),
        FUEL_CONSTANT.to_string(),
    )
    .await
    .map_err(|e| format!("Could not read {FUEL_CONSTANT} ({e}). Connect and load a tune first."))?;

    if baseline <= 0.0 {
        return Err(format!(
            "{FUEL_CONSTANT} reads {baseline}, which is not a usable baseline"
        ));
    }

    let enriched = (baseline * (1.0 + step_percent / 100.0) * 10.0).round() / 10.0;
    if enriched <= baseline {
        return Err(format!(
            "A {step_percent:.1}% step on {baseline} does not change the value at this \
             resolution. Use a larger step."
        ));
    }

    abort_flag().store(false, Ordering::SeqCst);

    emit(
        &app,
        DelayTestProgress {
            phase: "starting".into(),
            step: 0,
            total_steps: repeats,
            applied_value: enriched,
            baseline_value: baseline,
            message: format!(
                "{FUEL_CONSTANT} {baseline:.1} -> {enriched:.1} ({step_percent:.1}% richer), \
                 {repeats} steps, {hold_ms} ms hold. RAM only, never burned."
            ),
        },
    );

    // Anything that leaves this function must first put `baseline` back. The
    // helper is used on the happy path, on abort, and on error.
    async fn restore(state: &tauri::State<'_, AppState>, baseline: f64) -> Result<(), String> {
        crate::commands::constant_update::update_constant(
            state.clone(),
            FUEL_CONSTANT.to_string(),
            baseline,
        )
        .await
    }

    let mut completed = 0u32;
    for step in 1..=repeats {
        if abort_flag().load(Ordering::SeqCst) {
            break;
        }

        emit(
            &app,
            DelayTestProgress {
                phase: "enriching".into(),
                step,
                total_steps: repeats,
                applied_value: enriched,
                baseline_value: baseline,
                message: format!("step {step}/{repeats}: hold steady"),
            },
        );

        if let Err(e) = crate::commands::constant_update::update_constant(
            state.clone(),
            FUEL_CONSTANT.to_string(),
            enriched,
        )
        .await
        {
            // The write failed, so the ECU may or may not have taken it.
            // Restore regardless and stop.
            let restore_err = restore(&state, baseline).await.err();
            return Err(match restore_err {
                None => format!("Step {step} failed to apply ({e}). Baseline restored."),
                Some(r) => format!(
                    "Step {step} failed to apply ({e}) AND restoring {FUEL_CONSTANT} to \
                     {baseline} also failed ({r}). CYCLE THE KEY to reload the stored tune."
                ),
            });
        }

        tokio::time::sleep(Duration::from_millis(hold_ms)).await;

        if let Err(e) = restore(&state, baseline).await {
            return Err(format!(
                "Applied step {step} but could not restore {FUEL_CONSTANT} to {baseline} ({e}). \
                 The engine is running RICH. CYCLE THE KEY to reload the stored tune."
            ));
        }

        completed = step;

        emit(
            &app,
            DelayTestProgress {
                phase: "settling".into(),
                step,
                total_steps: repeats,
                applied_value: baseline,
                baseline_value: baseline,
                message: format!("step {step}/{repeats} done, settling"),
            },
        );

        if step < repeats {
            tokio::time::sleep(Duration::from_millis(settle_ms)).await;
        }
    }

    // Belt and braces: restore once more on the way out, in case the loop was
    // broken by an abort between the enrich and the restore.
    restore(&state, baseline).await.map_err(|e| {
        format!(
            "Test finished but the final restore of {FUEL_CONSTANT} to {baseline} failed ({e}). \
             CYCLE THE KEY to reload the stored tune."
        )
    })?;

    let aborted = abort_flag().load(Ordering::SeqCst);
    let summary = format!(
        "{} after {completed}/{repeats} steps. {FUEL_CONSTANT} restored to {baseline:.1}. \
         Nothing was burned.",
        if aborted { "Aborted" } else { "Completed" }
    );

    emit(
        &app,
        DelayTestProgress {
            phase: if aborted { "aborted" } else { "complete" }.into(),
            step: completed,
            total_steps: repeats,
            applied_value: baseline,
            baseline_value: baseline,
            message: summary.clone(),
        },
    );

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clamp is the safety boundary: no input may produce a lean step or an
    /// unbounded one. Mirrors the clamping in `run_afr_delay_test`.
    fn clamp_step(p: f64) -> f64 {
        p.abs().clamp(MIN_STEP_PERCENT, MAX_STEP_PERCENT)
    }

    #[test]
    fn negative_input_cannot_produce_a_lean_step() {
        // A negative percentage would lean the engine — the one outcome that
        // damages hardware. `abs` must make it an enrichment.
        assert_eq!(clamp_step(-8.0), 8.0);
        assert_eq!(clamp_step(-100.0), MAX_STEP_PERCENT);
        assert!(clamp_step(-0.001) >= MIN_STEP_PERCENT);
    }

    #[test]
    fn step_magnitude_is_bounded() {
        assert_eq!(clamp_step(1000.0), MAX_STEP_PERCENT);
        assert_eq!(clamp_step(0.5), MIN_STEP_PERCENT);
        assert_eq!(clamp_step(8.0), 8.0);
    }

    #[test]
    fn hold_and_settle_are_bounded() {
        assert_eq!(0u64.clamp(MIN_HOLD_MS, MAX_HOLD_MS), MIN_HOLD_MS);
        assert_eq!(u64::MAX.clamp(MIN_HOLD_MS, MAX_HOLD_MS), MAX_HOLD_MS);
    }

    /// The enriched value must round to the constant's 0.1 resolution and be
    /// strictly richer, otherwise the step is invisible to the ECU.
    #[test]
    fn enriched_value_rounds_to_resolution_and_is_richer() {
        let baseline: f64 = 12.6;
        let enriched = (baseline * 1.08 * 10.0).round() / 10.0;
        assert!(enriched > baseline, "must be richer");
        assert!((enriched - 13.6).abs() < 1e-9, "got {enriched}");
        // reqFuel is stored at 0.1 ms resolution, so a value that does not land
        // on that grid would be silently truncated by the ECU write.
        assert!(
            ((enriched * 10.0) - (enriched * 10.0).round()).abs() < 1e-9,
            "must land on the 0.1 ms grid"
        );
    }
}
