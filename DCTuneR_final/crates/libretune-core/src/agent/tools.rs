//! Tool definitions exposed to the LLM.
//!
//! Each tool here maps to an [`crate::action_scripting::Action`] variant (for
//! write tools) or to a read-side query (for read tools). The orchestrator
//! turns the model's [`crate::llm::types::ToolCall`]s into validated, clamped
//! proposals.
//!
//! Keeping the tool catalogue in one place lets us render a stable JSON-schema
//! description for the prompt regardless of provider.

use crate::llm::types::ToolDef;
use serde_json::json;

/// The set of tool names the assistant can expose. Kept as `&'static str` so
/// the orchestrator can match on them cheaply when dispatching tool calls.
pub mod tool_names {
    // Read tools
    pub const READ_TABLE: &str = "read_table";
    pub const READ_CONSTANT: &str = "read_constant";
    pub const LIST_TABLES: &str = "list_tables";
    pub const LIST_FEATURES: &str = "list_features";
    pub const SUMMARIZE_TUNE: &str = "summarize_tune_context";
    pub const TUNE_HEALTH: &str = "tune_health_check";

    // Propose tools (write, staged for approval — never applied directly)
    pub const PROPOSE_TABLE_EDIT: &str = "propose_table_edit";
    pub const PROPOSE_BULK_OP: &str = "propose_bulk_operation";
    pub const PROPOSE_CONSTANT_CHANGE: &str = "propose_constant_change";
}

/// Build the full catalogue of tools the model may call, as JSON-schema
/// [`ToolDef`]s. This is what the orchestrator attaches to every
/// [`crate::llm::types::ChatRequest`].
pub fn catalogue() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: tool_names::LIST_TABLES.into(),
            description: "List all editable tables with their role (Ve / Ignition / \
                          AfrTarget / ...) and dimensions. Call first to discover names."
                .into(),
            parameters: json!({"type": "object", "properties": {}}),
        },
        ToolDef {
            name: tool_names::READ_TABLE.into(),
            description: "Read the current values, axis bins, and units of one table.".into(),
            parameters: json!({
                "type": "object",
                "required": ["table_name"],
                "properties": {
                    "table_name": {"type": "string"}
                }
            }),
        },
        ToolDef {
            name: tool_names::READ_CONSTANT.into(),
            description: "Read a single constant's current value, min/max, units, and \
                          options (for bits-type feature toggles)."
                .into(),
            parameters: json!({
                "type": "object",
                "required": ["name"],
                "properties": {"name": {"type": "string"}}
            }),
        },
        ToolDef {
            name: tool_names::LIST_FEATURES.into(),
            description: "List feature-toggle (bits) constants and their available options.".into(),
            parameters: json!({"type": "object", "properties": {}}),
        },
        ToolDef {
            name: tool_names::SUMMARIZE_TUNE.into(),
            description: "Get an aggregated summary of a tune table: per-cell AFR error, \
                          data coverage, suspect/anomalous cells, and unexplored cells \
                          the model could estimate. VE tables only."
                .into(),
            parameters: json!({
                "type": "object",
                "required": ["table_name"],
                "properties": {
                    "table_name": {"type": "string"},
                    "region": {"type": "string", "description": "Optional region: idle|cruise|wot"}
                }
            }),
        },
        ToolDef {
            name: tool_names::TUNE_HEALTH.into(),
            description: "Get the overall health score and per-region coverage for a table.".into(),
            parameters: json!({
                "type": "object",
                "required": ["table_name"],
                "properties": {"table_name": {"type": "string"}}
            }),
        },
        ToolDef {
            name: tool_names::PROPOSE_TABLE_EDIT.into(),
            description: "Propose changing one cell of a table. The change is staged for \
                          explicit user approval — it is never applied automatically."
                .into(),
            parameters: json!({
                "type": "object",
                "required": ["table_name", "x_index", "y_index", "new_value"],
                "properties": {
                    "table_name": {"type": "string"},
                    "x_index": {"type": "integer", "description": "column (0-based)"},
                    "y_index": {"type": "integer", "description": "row (0-based)"},
                    "new_value": {"type": "number"},
                    "reason": {"type": "string"}
                }
            }),
        },
        ToolDef {
            name: tool_names::PROPOSE_BULK_OP.into(),
            description: "Propose a bulk operation (scale / smooth / interpolate / set_equal) \
                          over a set of cells. Staged for user approval."
                .into(),
            parameters: json!({
                "type": "object",
                "required": ["table_name", "operation", "cells"],
                "properties": {
                    "table_name": {"type": "string"},
                    "operation": {"type": "string", "enum": ["scale","smooth","interpolate","set_equal"]},
                    "cells": {"type": "array", "items": {"type": "array", "items": {"type": "integer"}, "maxItems": 2}},
                    "parameters": {"type": "object"},
                    "reason": {"type": "string"}
                }
            }),
        },
        ToolDef {
            name: tool_names::PROPOSE_CONSTANT_CHANGE.into(),
            description: "Propose changing a single constant / feature toggle. Staged for \
                          user approval; dangerous constants (pin assignments, limits) \
                          require explicit confirmation."
                .into(),
            parameters: json!({
                "type": "object",
                "required": ["name", "value"],
                "properties": {
                    "name": {"type": "string"},
                    "value": {"type": "number"},
                    "reason": {"type": "string"}
                }
            }),
        },
    ]
}
