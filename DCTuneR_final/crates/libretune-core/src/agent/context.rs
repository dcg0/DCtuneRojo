//! Context-gathering helpers for the agent loop.
//!
//! Builds the prompt payload the orchestrator sends to the LLM provider:
//! table snapshots, constant listings, and [`TuneContextSummary`]s. Kept
//! separate from [`crate::agent::orchestrator`] so context construction is
//! unit-testable without a live provider.

use crate::action_scripting::Action;
use crate::agent::summarize::{summarize_tune_context, TuneContextInputs, TuneContextSummary};
use crate::agent::tiers::{constant_safety_tier, ConstantSafetyTier};
use crate::ini::{Constant, EcuDefinition, TableDefinition, TableRole};
use serde::{Deserialize, Serialize};

/// A single constant as exposed to the model (name, units, range, role, tier).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantContext {
    pub name: String,
    pub label: Option<String>,
    pub units: String,
    pub min: f64,
    pub max: f64,
    pub current_value: Option<f64>,
    pub tier: ConstantSafetyTier,
    pub help: Option<String>,
}

/// A table as exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableContext {
    pub name: String,
    pub title: String,
    pub role: TableRole,
    pub dimensions: (usize, usize),
    pub x_label: Option<String>,
    pub y_label: Option<String>,
}

/// Gather the constant-context listing for a (filtered) set of constants.
///
/// `current_values` maps constant name -> current display value; missing
/// entries yield `current_value: None`. Callers typically pass only the
/// constants relevant to the current request to keep the payload small.
pub fn gather_constant_context<'a, I>(
    def: &EcuDefinition,
    names: I,
    current_values: &std::collections::HashMap<String, f64>,
) -> Vec<ConstantContext>
where
    I: IntoIterator<Item = &'a str>,
{
    names
        .into_iter()
        .filter_map(|name| {
            let c: &Constant = def.constants.get(name)?;
            Some(ConstantContext {
                name: c.name.clone(),
                label: c.label.clone(),
                units: c.units.clone(),
                min: c.min,
                max: c.max,
                current_value: current_values.get(name).copied(),
                tier: constant_safety_tier(&c.name),
                help: c.help.clone(),
            })
        })
        .collect()
}

/// Gather a compact listing of every table and its inferred role.
pub fn gather_table_context(def: &EcuDefinition) -> Vec<TableContext> {
    def.tables
        .values()
        .map(|t: &TableDefinition| TableContext {
            name: t.name.clone(),
            title: t.title.clone(),
            role: t.role,
            dimensions: (t.x_size, t.y_size),
            x_label: t.x_label.clone(),
            y_label: t.y_label.clone(),
        })
        .collect()
}

/// Build a [`TuneContextSummary`] for one table, forwarding to
/// [`summarize_tune_context`]. Convenience wrapper so the orchestrator has a
/// single import point.
pub fn summarize_table(
    table: &TableDefinition,
    inputs: &TuneContextInputs<'_>,
) -> TuneContextSummary {
    let role_str = match table.role {
        TableRole::Ve => "Ve",
        TableRole::Ignition => "Ignition",
        TableRole::AfrTarget => "AfrTarget",
        TableRole::WarmupEnrichment => "WarmupEnrichment",
        TableRole::Other => "Other",
    };
    summarize_tune_context(&table.name, role_str, inputs)
}

/// Group a flat list of [`Action`]s by the table or constant they touch, for
/// compact prompt rendering. Returns (target_name, count) tuples.
pub fn group_actions_by_target(actions: &[Action]) -> Vec<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for a in actions {
        let key = match a {
            Action::TableEdit { table_name, .. } | Action::BulkOperation { table_name, .. } => {
                format!("table:{}", table_name)
            }
            Action::ConstantChange { constant_name, .. } => {
                format!("const:{}", constant_name)
            }
            _ => continue,
        };
        *counts.entry(key).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}
