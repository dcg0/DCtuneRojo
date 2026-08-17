//! Constant safety tiering.
//!
//! [`Constant::min`] / [`Constant::max`] guarantee a value is *storable*, but
//! they cannot tell whether a storable-but-wrong value is *dangerous* (e.g.
//! re-assigning a coil output to a pin shared with the crank trigger can
//! damage hardware or prevent a restart). This module classifies constants by
//! safety tier so the AI assistant (and any other automation) can flag risky
//! configuration changes for explicit confirmation.
//!
//! The danger list is curated per ECU family. It is the one piece of genuinely
//! new domain knowledge in the assistant feature; everything else is wiring
//! of existing primitives. An unknown constant is treated as
//! [`ConstantSafetyTier::Caution`] (never silently applied as "safe").
//!
//! [`Constant::min`]: crate::ini::Constant::min
//! [`Constant::max`]: crate::ini::Constant::max

use serde::{Deserialize, Serialize};

/// How risky it is to change a given constant.
///
/// Ordered from least to most restrictive. The AI assistant surfaces the tier
/// on every proposed [`crate::action_scripting::Action::ConstantChange`] and
/// gates dangerous ones behind an explicit per-item confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConstantSafetyTier {
    /// Benign runtime/comfort setting (e.g. display units, gauge config).
    /// No engine-behavior impact.
    Safe,
    /// Affects engine calibration (fuel, ignition, idle, boost targets) but
    /// is reversible and bounded by `min`/`max`. Review recommended. This is
    /// also the default tier for unknown constants — anything not clearly
    /// safe is never silently applied.
    #[default]
    Caution,
    /// Can alter engine behavior in ways that `min`/`max` cannot protect
    /// against: I/O pin assignment, trigger/decoder config, output inversion,
    /// rev/boost limits set to extreme values. Requires explicit confirmation.
    Dangerous,
}

/// Substrings (lowercased) that mark a constant as [`ConstantSafetyTier::Dangerous`],
/// keyed by ECU family signature prefix. Matched against the constant *name*.
///
/// This is deliberately conservative: false positives (flagging a safe
/// constant) only cost the user a confirmation click, while false negatives
/// (treating a dangerous constant as safe) can brick hardware.
fn dangerous_name_substrings() -> Vec<&'static str> {
    // These keywords are common across Speeduino / rusEFI / FOME / epicEFI /
    // MegaSquirt INIs. ECU-family-specific overrides can be added later
    // without changing the public API.
    vec![
        // Output / pin assignment
        "outputpin",
        "outputpinnumber",
        "pin",
        "inversion",
        "invert",
        // Trigger / decoder
        "triggertype",
        "triggerpattern",
        "triggerspeed",
        "decoder",
        "primarytrigger",
        "secondarytrigger",
        // Hardware protection limits
        "boostcut",
        "overboost",
        "maxrpm",
        "revlimit",
        "hardrevlimit",
        "maxduty",
        // Injection / ignition hardware
        "injopen",
        "ignfire",
        "coilcharge",
        // Cranking / startup safety
        "crankingpwm",
        "fuelprime",
    ]
}

/// Substrings that mark a constant as [`ConstantSafetyTier::Safe`].
fn safe_name_substrings() -> Vec<&'static str> {
    vec![
        // Display / UI
        "units",
        "display",
        "language",
        // Communication-only
        "baud",
        "bluetooth",
    ]
}

/// Classify a constant by name into a safety tier.
///
/// The classification is heuristic (name-substring based) and conservative:
/// anything not clearly safe is treated as [`ConstantSafetyTier::Caution`],
/// and anything matching a hardware-affecting keyword is
/// [`ConstantSafetyTier::Dangerous`]. Dangerous wins over safe if both match.
///
/// Callers that have richer per-platform knowledge should prefer their own
/// classification and only fall back to this for unknown constants.
pub fn constant_safety_tier(constant_name: &str) -> ConstantSafetyTier {
    let lname = constant_name.to_lowercase();

    // Dangerous takes precedence: if a name looks hardware-affecting at all,
    // treat it as dangerous regardless of whether it also matched "safe".
    for needle in dangerous_name_substrings() {
        if lname.contains(needle) {
            return ConstantSafetyTier::Dangerous;
        }
    }
    for needle in safe_name_substrings() {
        if lname.contains(needle) {
            return ConstantSafetyTier::Safe;
        }
    }
    ConstantSafetyTier::Caution
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_keywords_flag() {
        assert_eq!(
            constant_safety_tier("boostCutPressure"),
            ConstantSafetyTier::Dangerous
        );
        assert_eq!(
            constant_safety_tier("maxRpm"),
            ConstantSafetyTier::Dangerous
        );
        assert_eq!(
            constant_safety_tier("fanOutputPin"),
            ConstantSafetyTier::Dangerous
        );
        assert_eq!(
            constant_safety_tier("inversion"),
            ConstantSafetyTier::Dangerous
        );
    }

    #[test]
    fn safe_keywords_flag() {
        assert_eq!(
            constant_safety_tier("displayUnits"),
            ConstantSafetyTier::Safe
        );
        assert_eq!(
            constant_safety_tier("canBaudRate"),
            ConstantSafetyTier::Safe
        );
    }

    #[test]
    fn unknown_defaults_to_caution() {
        // A fuel-related constant that isn't in either list.
        assert_eq!(constant_safety_tier("reqFuel"), ConstantSafetyTier::Caution);
        assert_eq!(
            constant_safety_tier("idleRpmTarget"),
            ConstantSafetyTier::Caution
        );
    }

    #[test]
    fn dangerous_wins_over_safe() {
        // "pin" is dangerous even though it might co-occur with safe substrings.
        assert_eq!(
            constant_safety_tier("displayPinConfig"),
            ConstantSafetyTier::Dangerous
        );
    }
}
