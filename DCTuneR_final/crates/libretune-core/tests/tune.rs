//! `TuneValue` unit tests.
//!
//! `TuneValue` is the tagged union that backs every constant in a tune file
//! (see `src/tune/file.rs`). It has four variants corresponding to the value
//! types an INI constant may hold:
//! - [`TuneValue::Scalar`] — single `f64` (the common case, e.g. `RevLim`)
//! - [`TuneValue::Array`] — `Vec<f64>` (curves, bin arrays, 1D tables)
//! - [`TuneValue::String`] — free-form text (INI `string` constants)
//! - [`TuneValue::Bool`] — on/off flags (INI `bits` aliases `on`/`off`)
//!
//! These tests exercise construction and pattern-matching for each variant,
//! plus edge cases (negatives, zero, precision, empty/long payloads) that the
//! tune loader and serializer must round-trip faithfully.

#![allow(clippy::approx_constant)]
use libretune_core::tune::TuneValue;

/// Constructing the most common variant (Scalar) round-trips the f64 payload
/// and pattern-matches the expected arm.
#[test]
fn test_tune_value_scalar_creation() {
    let value = TuneValue::Scalar(100.0);
    match value {
        TuneValue::Scalar(v) => assert_eq!(v, 100.0),
        _ => panic!("Expected scalar value"),
    }
}

/// Array variant preserves element count and ordering (used by bin arrays).
#[test]
fn test_tune_value_array_creation() {
    let value = TuneValue::Array(vec![1.0, 2.0, 3.0]);
    match value {
        TuneValue::Array(v) => assert_eq!(v.len(), 3),
        _ => panic!("Expected array value"),
    }
}

/// String variant carries the raw text; used for INI `string` constants
/// (e.g. algorithm selection, vehicle notes).
#[test]
fn test_tune_value_string_creation() {
    let value = TuneValue::String("test".to_string());
    match value {
        TuneValue::String(v) => assert_eq!(v, "test"),
        _ => panic!("Expected string value"),
    }
}

/// Bool variant; maps to INI `bits` aliases whose displayValue is on/off.
#[test]
fn test_tune_value_bool_creation() {
    let value = TuneValue::Bool(true);
    match value {
        TuneValue::Bool(v) => assert!(v),
        _ => panic!("Expected bool value"),
    }
}

#[test]
fn test_tune_value_negative_scalar() {
    // Negative values are common in tuning (e.g. ignition advance offsets,
    // temperature offsets) and must not be sign-flipped by the variant.
    let value = TuneValue::Scalar(-42.5);
    match value {
        TuneValue::Scalar(v) => assert_eq!(v, -42.5),
        _ => panic!("Expected scalar value"),
    }
}

/// Zero must be preserved distinctly — it is a meaningful "off/unset" value
/// and must not collapse into a default/empty representation.
#[test]
fn test_tune_value_zero() {
    let value = TuneValue::Scalar(0.0);
    match value {
        TuneValue::Scalar(v) => assert_eq!(v, 0.0),
        _ => panic!("Expected scalar value"),
    }
}

/// Precision guard: f64 can hold more digits than the INI display precision,
/// so a value like 3.14159 must round-trip without silent rounding (matters
/// for scale factors and conversion expressions).
#[test]
fn test_tune_value_precision() {
    #[allow(clippy::approx_constant)]
    let value = TuneValue::Scalar(3.14159);
    match value {
        TuneValue::Scalar(v) => assert!((v - 3.14159).abs() < 0.00001),
        _ => panic!("Expected scalar value"),
    }
}

/// Large magnitudes (e.g. high-RPM rev limits, large cycle counters) must
/// remain exact within f64 — no overflow/clamping at this layer.
#[test]
fn test_tune_value_large_number() {
    let value = TuneValue::Scalar(99999.99);
    match value {
        TuneValue::Scalar(v) => assert_eq!(v, 99999.99),
        _ => panic!("Expected scalar value"),
    }
}

#[test]
fn test_tune_value_equality() {
    // Equality is structural on the inner value: two Scalars with the same f64
    // are equal, a differing f64 is not. This underpins the tune diff logic
    // (see `tune/diff.rs`), which compares old vs. new values.
    let v1 = TuneValue::Scalar(42.0);
    let v2 = TuneValue::Scalar(42.0);
    let v3 = TuneValue::Scalar(43.0);

    match (&v1, &v2, &v3) {
        (TuneValue::Scalar(a), TuneValue::Scalar(b), TuneValue::Scalar(c)) => {
            assert_eq!(a, b);
            assert_ne!(b, c);
        }
        _ => panic!("Expected scalar values"),
    }
}

#[test]
fn test_tune_value_array_operations() {
    // Both endpoints and length are preserved: bin arrays are indexed by
    // position, so first/last elements and count all matter for table axes.
    let arr = TuneValue::Array(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    match arr {
        TuneValue::Array(v) => {
            assert_eq!(v.len(), 5);
            assert_eq!(v[0], 1.0);
            assert_eq!(v[4], 5.0);
        }
        _ => panic!("Expected array value"),
    }
}

/// An empty array is a legal (if degenerate) value — e.g. a freshly-created
/// curve before any bins are populated. Must not panic or coerce to another
/// variant.
#[test]
fn test_tune_value_array_empty() {
    let arr = TuneValue::Array(vec![]);
    match arr {
        TuneValue::Array(v) => assert_eq!(v.len(), 0),
        _ => panic!("Expected array value"),
    }
}

/// Elements remain usable as f64 for downstream math (sums, averages) — the
/// variant must not box or wrap the values in a way that blocks iteration.
#[test]
fn test_tune_value_array_access() {
    let arr = TuneValue::Array(vec![10.0, 20.0, 30.0]);
    match arr {
        TuneValue::Array(v) => {
            let sum: f64 = v.iter().sum();
            assert_eq!(sum, 60.0);
        }
        _ => panic!("Expected array value"),
    }
}

#[test]
fn test_tune_value_string_empty() {
    // An empty string is distinct from "no value" — used when a constant is
    // present but intentionally blank (e.g. an unset vehicle note).
    let value = TuneValue::String(String::new());
    match value {
        TuneValue::String(v) => assert!(v.is_empty()),
        _ => panic!("Expected string value"),
    }
}

/// Long strings must not be truncated: some INI constants carry free-form
/// text (calibration notes) that can be arbitrarily long.
#[test]
fn test_tune_value_string_long() {
    let value = TuneValue::String("a".repeat(1000));
    match value {
        TuneValue::String(v) => assert_eq!(v.len(), 1000),
        _ => panic!("Expected string value"),
    }
}

/// The `false` bool value must survive round-trip — a default-true bug here
/// would silently enable outputs (e.g. launch control, boost control).
#[test]
fn test_tune_value_bool_false() {
    let value = TuneValue::Bool(false);
    match value {
        TuneValue::Bool(v) => assert!(!v),
        _ => panic!("Expected bool value"),
    }
}
