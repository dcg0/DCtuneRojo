//! Port Editor (I/O pin assignment) persistence layer.
//!
//! Stores per-INI port assignments in `<project>/projectCfg/port_editor.json`.

use crate::paths::get_port_editor_store_path;
use libretune_core::project::Project;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortEditorAssignment {
    pub id: String,
    pub name: String,
    pub physical_pin: String,
    pub function: String,
    pub channel: u32,
    pub inverted: bool,
    pub pullup: bool,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PortEditorStore {
    pub assignments: HashMap<String, Vec<PortEditorAssignment>>,
}

pub fn load_port_editor_store(project: &Project) -> Result<PortEditorStore, String> {
    let path = get_port_editor_store_path(project);
    if !path.exists() {
        return Ok(PortEditorStore::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read port editor store: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse port editor store: {}", e))
}

pub fn save_port_editor_store(project: &Project, store: &PortEditorStore) -> Result<(), String> {
    let path = get_port_editor_store_path(project);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create port editor directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize port editor store: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write port editor store: {}", e))?;
    Ok(())
}
