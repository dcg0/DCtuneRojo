//! Acceleration-enrichment (AE) autotune analyzer.
//!
//! Speeduino's TPS-based AE adds `taeRates[bin]%` extra fuel (PW adder) for
//! `aeTime` ms on a tip-in, where `bin` is chosen from peak TPS-DOT against
//! `taeBins`. If a tip-in still runs lean the enrichment was too weak; if rich,
//! too strong. This recovers that per-bin error from ordinary logs so the curve
//! can be tuned without a dedicated procedure.
//!
//! Three things make it more than a naive AFR read, and they are the whole
//! reason it works:
//!
//! 1. **Transport delay.** The wideband sees a tip-in one transport delay later
//!    (~0.7–1 s on an NA MX-5). The AFR excursion is measured over a window
//!    shifted by `delay_ms`; without it the sign of the correction inverts.
//! 2. **Baseline subtraction.** During a tip-in the VE-table error at the moving
//!    operating point is also present. Taking the response-window mean *minus*
//!    the local baseline (samples just before and just after) leaves the
//!    AE-attributable residual. The mean (not the max) keeps the metric
//!    unbiased — random noise averages back to the baseline.
//! 3. **Evidence gating.** Tip-ins are sparse and the top TPS-DOT bins may be
//!    physically unreachable on a cable throttle. Every bin is gated on a
//!    minimum event count; a bin the data can't see is held, never invented.
//!
//! The analyzer only recommends; nothing is written to the ECU here.

/// One log/realtime sample the analyzer consumes.
#[derive(Debug, Clone, Copy)]
pub struct AeSample {
    pub timestamp_ms: u64,
    /// TPS rate of change, %/s (the AE input).
    pub tps_dot: f64,
    pub afr: f64,
    /// Target AFR at this sample (0 → use `AeConfig::target_afr_fallback`).
    pub afr_target: f64,
    /// Coolant temp in the log's own units; compared against `min_clt`.
    pub clt: f64,
}

/// Analyzer configuration. Bins/rates come from the tune; the rest are analysis
/// knobs with sensible defaults.
#[derive(Debug, Clone)]
pub struct AeConfig {
    pub tae_bins: Vec<f64>,
    pub tae_rates: Vec<f64>,
    pub ae_time_ms: f64,
    /// AFR transport delay used to align the response window (from the delay
    /// measurement / lambda-delay setting).
    pub delay_ms: f64,
    /// TPS-DOT above which a sample begins a tip-in event (%/s).
    pub event_floor: f64,
    /// Warm-engine filter; samples below this coolant temp are ignored.
    pub min_clt: f64,
    /// Minimum tip-in events in a bin before it may be moved.
    pub min_events: usize,
    /// Largest change to a `taeRate` per pass (percentage points).
    pub authority_pct: f64,
    /// % PW-adder change per 1.0 AFR of residual error. ~100/target: a lean of
    /// `d` AFR at target `T` is roughly a `d/T` fuel shortfall.
    pub rate_gain_pct_per_afr: f64,
    pub target_afr_fallback: f64,
}

impl Default for AeConfig {
    fn default() -> Self {
        Self {
            tae_bins: vec![60.0, 160.0, 400.0, 600.0],
            tae_rates: vec![10.0, 13.0, 17.0, 20.0],
            ae_time_ms: 160.0,
            delay_ms: 800.0,
            event_floor: 30.0,
            min_clt: 70.0,
            min_events: 8,
            authority_pct: 5.0,
            rate_gain_pct_per_afr: 100.0 / 14.7,
            target_afr_fallback: 14.7,
        }
    }
}

/// A detected tip-in and its AE residual (+ = lean = enrichment too weak).
#[derive(Debug, Clone, Copy)]
pub struct AeEvent {
    pub peak_tps_dot: f64,
    pub residual_afr: f64,
}

/// Per-bin recommendation. `recommended_rate == current_rate` when the bin is
/// held (insufficient data or already on target).
#[derive(Debug, Clone)]
pub struct AeBinRecommendation {
    pub bin: usize,
    pub lo: f64,
    pub hi: Option<f64>,
    pub events: usize,
    pub median_residual_afr: Option<f64>,
    pub current_rate: f64,
    pub recommended_rate: f64,
    pub sufficient_data: bool,
    pub note: String,
}

fn afr_err(s: &AeSample, fallback: f64) -> f64 {
    let t = if s.afr_target > 0.0 {
        s.afr_target
    } else {
        fallback
    };
    s.afr - t
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

fn mean(v: &[f64]) -> Option<f64> {
    if v.is_empty() {
        None
    } else {
        Some(v.iter().sum::<f64>() / v.len() as f64)
    }
}

/// Which `taeBins` bin a peak TPS-DOT falls in, or `None` if below the first.
pub fn bin_of(peak_tps_dot: f64, tae_bins: &[f64]) -> Option<usize> {
    let mut b = None;
    for (i, &thr) in tae_bins.iter().enumerate() {
        if peak_tps_dot >= thr {
            b = Some(i);
        }
    }
    b
}

/// Detect warm tip-in events in a single time-ordered log and measure each
/// one's AE residual. `samples` must be sorted by `timestamp_ms`.
pub fn analyze_events(samples: &[AeSample], cfg: &AeConfig) -> Vec<AeEvent> {
    let n = samples.len();
    let first_bin = cfg.tae_bins.first().copied().unwrap_or(60.0);
    let settle_ms = cfg.ae_time_ms + 500.0;
    let base_span_ms = 400.0;
    let mut out = Vec::new();

    let mut i = 1;
    while i < n {
        if samples[i].tps_dot < cfg.event_floor {
            i += 1;
            continue;
        }
        // Event runs while TPS-DOT stays above half the floor; track the peak.
        let start = i;
        let mut j = i;
        let mut peak = samples[i].tps_dot;
        let mut peak_i = i;
        while j < n && samples[j].tps_dot >= cfg.event_floor * 0.5 {
            if samples[j].tps_dot > peak {
                peak = samples[j].tps_dot;
                peak_i = j;
            }
            j += 1;
        }

        if samples[peak_i].clt >= cfg.min_clt && peak >= first_bin {
            // Delay-shifted response window, and a local baseline bracketing it.
            let peak_ts = samples[peak_i].timestamp_ms as f64;
            let start_ts = samples[start].timestamp_ms as f64;
            let resp_lo = peak_ts + cfg.delay_ms;
            let resp_hi = resp_lo + settle_ms;

            let mut resp = Vec::new();
            let mut base = Vec::new();
            for s in samples {
                let ts = s.timestamp_ms as f64;
                if ts >= resp_lo && ts <= resp_hi {
                    resp.push(afr_err(s, cfg.target_afr_fallback));
                } else if (ts >= start_ts - base_span_ms && ts < start_ts)
                    || (ts > resp_hi && ts <= resp_hi + base_span_ms)
                {
                    base.push(afr_err(s, cfg.target_afr_fallback));
                }
            }
            if resp.len() >= 3 && base.len() >= 3 {
                if let (Some(rm), Some(bm)) = (mean(&resp), median(base)) {
                    out.push(AeEvent {
                        peak_tps_dot: peak,
                        residual_afr: rm - bm,
                    });
                }
            }
        }
        i = j + 1;
    }
    out
}

/// Bucket events by `taeBins` and produce evidence-gated recommendations, one
/// per bin. A bin below `min_events` is held with an "insufficient data" note.
pub fn recommend(events: &[AeEvent], cfg: &AeConfig) -> Vec<AeBinRecommendation> {
    let nbins = cfg.tae_bins.len();
    let mut per_bin: Vec<Vec<f64>> = vec![Vec::new(); nbins];
    for e in events {
        if let Some(b) = bin_of(e.peak_tps_dot, &cfg.tae_bins) {
            per_bin[b].push(e.residual_afr);
        }
    }

    (0..nbins)
        .map(|b| {
            let lo = cfg.tae_bins[b];
            let hi = cfg.tae_bins.get(b + 1).copied();
            let cur = cfg.tae_rates.get(b).copied().unwrap_or(0.0);
            let residuals = &per_bin[b];
            let count = residuals.len();
            let med = median(residuals.clone());

            if count < cfg.min_events {
                AeBinRecommendation {
                    bin: b,
                    lo,
                    hi,
                    events: count,
                    median_residual_afr: med,
                    current_rate: cur,
                    recommended_rate: cur,
                    sufficient_data: false,
                    note: format!("insufficient data (need {}) — held", cfg.min_events),
                }
            } else {
                let m = med.unwrap_or(0.0);
                // + residual = lean = enrichment too weak = richen (raise rate).
                let delta =
                    (m * cfg.rate_gain_pct_per_afr).clamp(-cfg.authority_pct, cfg.authority_pct);
                let rec = (cur + delta).max(0.0);
                let note = if delta > 0.3 {
                    format!("richen (+{:.1}); tip-ins run {:+.2} AFR lean", delta, m)
                } else if delta < -0.3 {
                    format!("lean ({:.1}); tip-ins run {:+.2} AFR rich", delta, m)
                } else {
                    format!("on target ({:+.2} AFR); no change", m)
                };
                AeBinRecommendation {
                    bin: b,
                    lo,
                    hi,
                    events: count,
                    median_residual_afr: med,
                    current_rate: cur,
                    recommended_rate: rec,
                    sufficient_data: true,
                    note,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a synthetic warm log: `secs` of steady idle, one tip-in at `t_ms`
    /// with `peak` TPS-DOT, and an AFR excursion of `resp_afr` (relative to
    /// target) appearing `delay_ms` after the tip-in for `ae_time+` ms.
    fn synth(peak: f64, resp_afr: f64, delay_ms: u64) -> Vec<AeSample> {
        let dt = 100u64; // 10 Hz
        let target = 14.7;
        let tip_at = 3000u64;
        let mut v = Vec::new();
        let mut t = 0u64;
        while t <= 8000 {
            let dt_since_resp = t as i64 - (tip_at + delay_ms) as i64;
            let in_response = (0..660).contains(&dt_since_resp); // aeTime+settle-ish
            let tps_dot = if t == tip_at { peak } else { 0.0 };
            let afr = target + if in_response { resp_afr } else { 0.0 };
            v.push(AeSample {
                timestamp_ms: t,
                tps_dot,
                afr,
                afr_target: target,
                clt: 85.0,
            });
            t += dt;
        }
        v
    }

    #[test]
    fn lean_tip_in_recommends_richer() {
        let cfg = AeConfig {
            min_events: 1,
            delay_ms: 800.0,
            ..Default::default()
        };
        // +0.6 AFR lean during the delayed response window -> too weak -> richen.
        let ev = analyze_events(&synth(100.0, 0.6, 800), &cfg);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].residual_afr > 0.4, "residual {}", ev[0].residual_afr);
        let recs = recommend(&ev, &cfg);
        assert_eq!(bin_of(100.0, &cfg.tae_bins), Some(0));
        assert!(recs[0].sufficient_data);
        assert!(
            recs[0].recommended_rate > recs[0].current_rate,
            "lean tip-in must richen: {} -> {}",
            recs[0].current_rate,
            recs[0].recommended_rate
        );
    }

    #[test]
    fn rich_tip_in_recommends_leaner() {
        let cfg = AeConfig {
            min_events: 1,
            delay_ms: 800.0,
            ..Default::default()
        };
        let ev = analyze_events(&synth(100.0, -0.6, 800), &cfg);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].residual_afr < -0.4);
        let recs = recommend(&ev, &cfg);
        assert!(recs[0].recommended_rate < recs[0].current_rate);
    }

    #[test]
    fn wrong_delay_reads_no_excursion() {
        // The excursion sits at +800 ms; reading with 0 ms delay must not see
        // it as the tip-in signal (this is why delay correction matters).
        let cfg = AeConfig {
            min_events: 1,
            delay_ms: 0.0,
            ..Default::default()
        };
        let ev = analyze_events(&synth(100.0, 0.6, 800), &cfg);
        // Either no measurable event, or a residual far smaller than the true 0.6.
        assert!(ev.is_empty() || ev[0].residual_afr.abs() < 0.3);
    }

    #[test]
    fn unreachable_bins_are_held_not_invented() {
        let cfg = AeConfig {
            min_events: 8,
            delay_ms: 800.0,
            ..Default::default()
        };
        // Only bin-0 events (peak 100). Bins 1-3 have none.
        let mut all = Vec::new();
        for _ in 0..12 {
            all.extend(analyze_events(&synth(100.0, 0.5, 800), &cfg));
        }
        let recs = recommend(&all, &cfg);
        assert!(recs[0].sufficient_data, "bin 0 has 12 events");
        for r in &recs[1..] {
            assert!(!r.sufficient_data, "empty bins must be insufficient");
            assert_eq!(
                r.recommended_rate, r.current_rate,
                "an unseen bin must be held, not moved"
            );
        }
    }
}
