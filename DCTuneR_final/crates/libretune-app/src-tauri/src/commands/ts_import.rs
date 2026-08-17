//! TunerStudio project import preview & import commands.

use crate::state::AppState;
use crate::{ConnectionSettingsResponse, CurrentProjectInfo};
use libretune_core::project::Project;
use serde::Serialize;

/// Preview data for a TS project import
#[derive(Debug, Clone, Serialize)]
pub struct TsImportPreview {
    pub project_name: String,
    pub ini_file: Option<String>,
    pub has_tune: bool,
    pub restore_point_count: usize,
    pub has_pc_variables: bool,
    pub connection_port: Option<String>,
    pub connection_baud: Option<u32>,
}

/// Preview a TS project before importing
#[tauri::command]
pub async fn preview_tunerstudio_import(path: String) -> Result<TsImportPreview, String> {
    use libretune_core::project::Properties;

    let ts_path = std::path::Path::new(&path);

    // Look for project.properties in projectCfg subfolder
    let project_props_path = ts_path.join("projectCfg").join("project.properties");
    if !project_props_path.exists() {
        return Err("Not a valid TS project: project.properties not found".to_string());
    }

    let project_props = Properties::load(&project_props_path)
        .map_err(|e| format!("Failed to read project: {}", e))?;

    // Extract project name
    let project_name = project_props
        .get("projectName")
        .cloned()
        .unwrap_or_else(|| {
            ts_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Imported Project".to_string())
        });

    let ini_file = project_props.get("ecuConfigFile").cloned();

    let tune_path = ts_path.join("CurrentTune.msq");
    let has_tune = tune_path.exists();

    let restore_dir = ts_path.join("restorePoints");
    let restore_point_count = if restore_dir.exists() {
        std::fs::read_dir(&restore_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "msq"))
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };

    let pc_path = ts_path.join("projectCfg").join("pcVariableValues.msq");
    let has_pc_variables = pc_path.exists();

    let connection_port = project_props.get("commPort").cloned();
    let connection_baud = project_props.get_i32("baudRate").map(|v| v as u32);

    Ok(TsImportPreview {
        project_name,
        ini_file,
        has_tune,
        restore_point_count,
        has_pc_variables,
        connection_port,
        connection_baud,
    })
}

/// Import a TS project
#[tauri::command]
pub async fn import_tunerstudio_project(
    state: tauri::State<'_, AppState>,
    source_path: String,
) -> Result<CurrentProjectInfo, String> {
    let project = Project::import_tunerstudio(&source_path, None)
        .map_err(|e| format!("Failed to import TS project: {}", e))?;

    let response = CurrentProjectInfo {
        name: project.config.name.clone(),
        path: project.path.to_string_lossy().to_string(),
        signature: project.config.signature.clone(),
        has_tune: project.current_tune.is_some(),
        tune_modified: project.dirty,
        connection: ConnectionSettingsResponse {
            port: project.config.connection.port.clone(),
            baud_rate: project.config.connection.baud_rate,
            auto_connect: project.config.settings.auto_connect,
        },
    };

    let mut proj_guard = state.current_project.lock().await;
    *proj_guard = Some(project);

    Ok(response)
}
