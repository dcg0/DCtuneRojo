//! Extended table operation tests.
//!
//! Supplements `tests/table_ops.rs` with focused tests on selection-scoped
//! operations: offsetting, filling, and linear interpolation across a region.
//! Note the convention that interpolation anchors are the EDGES of the
//! selection (not its centre), which is asserted in the interpolation cases.

use libretune_core::table_ops::{
    add_offset, fill_region, interpolate_linear, FillDirection, InterpolationAxis,
};

#[test]
fn test_add_offset() {
    let z_values = vec![vec![10.0, 20.0, 30.0], vec![40.0, 50.0, 60.0]];
    let selected_cells = vec![(0, 0), (1, 1)];

    let result = add_offset(&z_values, selected_cells, 5.0);

    assert_eq!(result[0][0], 15.0);
    assert_eq!(result[0][1], 20.0); // Unchanged
    assert_eq!(result[1][1], 55.0);
}

#[test]
fn test_add_offset_negative() {
    let z_values = vec![vec![10.0, 20.0, 30.0]];
    let selected_cells = vec![(0, 1)];

    let result = add_offset(&z_values, selected_cells, -5.0);

    assert_eq!(result[0][1], 15.0);
}

#[test]
fn test_interpolate_linear_row() {
    let z_values = vec![vec![10.0, 0.0, 0.0, 50.0], vec![20.0, 0.0, 0.0, 80.0]];

    // Select the first row
    let selected_cells = vec![(0, 0), (0, 1), (0, 2), (0, 3)];

    let result = interpolate_linear(&z_values, selected_cells, InterpolationAxis::Row);

    // Row 0 should be interpolated
    assert_eq!(result[0][0], 10.0);
    assert!((result[0][1] - 23.333).abs() < 0.001); // 10 + (40 * 1/3)
    assert!((result[0][2] - 36.666).abs() < 0.001); // 10 + (40 * 2/3)
    assert_eq!(result[0][3], 50.0);

    // Row 1 should be unchanged
    assert_eq!(result[1][1], 0.0);
}

#[test]
fn test_interpolate_linear_col() {
    let z_values = vec![
        vec![10.0, 20.0],
        vec![0.0, 0.0],
        vec![0.0, 0.0],
        vec![40.0, 80.0],
    ];

    // Select column 1 (values 20, 0, 0, 80)
    let selected_cells = vec![(0, 1), (1, 1), (2, 1), (3, 1)];

    let result = interpolate_linear(&z_values, selected_cells, InterpolationAxis::Col);

    // Col 0 unchanged
    assert_eq!(result[1][0], 0.0);

    // Col 1 interpolated
    assert_eq!(result[0][1], 20.0);
    assert!((result[1][1] - 40.0).abs() < 0.001); // 20 + 20
    assert!((result[2][1] - 60.0).abs() < 0.001); // 20 + 40
    assert_eq!(result[3][1], 80.0);
}

#[test]
fn test_interpolate_linear_partial_selection() {
    let z_values = vec![vec![10.0, 0.0, 0.0, 40.0]];
    // Only select middle two, but min/max X will span full range of selection
    // NOTE: Current implementation of interpolate_linear determines min/max of selection
    // and interpolates between those bounds.
    let selected_cells = vec![(0, 1), (0, 2)];

    // In this case, min_x=1, max_x=2. It interpolates between index 1 and 2 using bounds 1 and 2.
    // value at 1 is 0.0, value at 2 is 0.0. Result should be 0.0.
    // This highlights behavior: interpolation anchors are the EDGES of the selection.

    let result = interpolate_linear(&z_values, selected_cells, InterpolationAxis::Row);
    assert_eq!(result[0][1], 0.0);
}

#[test]
fn test_fill_right() {
    let z_values = vec![vec![10.0, 0.0, 0.0], vec![20.0, 0.0, 0.0]];

    let selected_cells = vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)]; // Full 2x3 block selection

    let result = fill_region(&z_values, selected_cells, FillDirection::Right);

    // Should copy column 0 to 1 and 2
    assert_eq!(result[0][1], 10.0);
    assert_eq!(result[0][2], 10.0);
    assert_eq!(result[1][1], 20.0);
    assert_eq!(result[1][2], 20.0);
}

#[test]
fn test_fill_down() {
    let z_values = vec![
        vec![10.0, 20.0, 30.0],
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
    ];

    let selected_cells = vec![
        (0, 0),
        (0, 1),
        (0, 2),
        (1, 0),
        (1, 1),
        (1, 2),
        (2, 0),
        (2, 1),
        (2, 2),
    ]; // Full 3x3 block

    let result = fill_region(&z_values, selected_cells, FillDirection::Down);

    // Should copy row 0 to 1 and 2
    assert_eq!(result[1][0], 10.0);
    assert_eq!(result[2][0], 10.0);
    assert_eq!(result[1][1], 20.0);
    assert_eq!(result[2][2], 30.0);
}

// Regression tests (2026-08-01): a selection referencing cells outside the
// table's actual dimensions — e.g. a stale multi-cell selection left over
// from before rebin_table shrank the table — must be a safe no-op, not a
// panic. Every other function in table_ops.rs already bounds-checks via
// get_cell_value(); these two indexed directly and didn't.

#[test]
fn test_interpolate_linear_out_of_bounds_row_is_noop_not_panic() {
    let z_values = vec![vec![10.0, 20.0, 30.0], vec![40.0, 50.0, 60.0]];
    // Row 5 doesn't exist (table only has 2 rows).
    let selected_cells = vec![(5, 0), (5, 2)];

    let result = interpolate_linear(&z_values, selected_cells, InterpolationAxis::Row);

    assert_eq!(
        result, z_values,
        "out-of-range selection must leave the table unchanged"
    );
}

#[test]
fn test_interpolate_linear_out_of_bounds_col_is_noop_not_panic() {
    let z_values = vec![vec![10.0, 20.0], vec![30.0, 40.0]];
    // Column 9 doesn't exist (table only has 2 columns).
    let selected_cells = vec![(0, 9), (1, 9)];

    let result = interpolate_linear(&z_values, selected_cells, InterpolationAxis::Col);

    assert_eq!(
        result, z_values,
        "out-of-range selection must leave the table unchanged"
    );
}

#[test]
fn test_fill_region_out_of_bounds_is_noop_not_panic() {
    let z_values = vec![vec![10.0, 0.0], vec![20.0, 0.0]];
    // Row 3 doesn't exist (table only has 2 rows) — this is exactly the
    // kind of stale selection a rebin_table shrink would produce.
    let selected_cells = vec![(0, 0), (0, 1), (3, 0), (3, 1)];

    let result = fill_region(&z_values, selected_cells, FillDirection::Right);

    assert_eq!(
        result, z_values,
        "out-of-range selection must leave the table unchanged"
    );
}

#[test]
fn test_fill_region_partially_out_of_bounds_is_noop() {
    // Selection is mostly valid but includes one cell past the last column —
    // the whole operation must be rejected rather than partially applied.
    let z_values = vec![vec![10.0, 0.0, 0.0]];
    let selected_cells = vec![(0, 0), (0, 1), (0, 2), (0, 99)];

    let result = fill_region(&z_values, selected_cells, FillDirection::Right);

    assert_eq!(result, z_values);
}
