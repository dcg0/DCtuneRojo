//! Cross-platform application path helpers.
//!
//! All helpers fall back to platform-default data dirs via the `dirs` crate
//! if the Tauri path resolver fails.

use libretune_core::project::Project;
use std::path::PathBuf;
use tauri::Manager;

/// Get the LibreTune app data directory (cross-platform).
pub fn get_app_data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("LibreTune")
    })
}

/// Get the projects directory (cross-platform).
///
/// Projects are created and imported by `libretune-core`, which resolves this
/// location via [`Project::projects_dir`] (`~/Documents/LibreTuneProjects`).
/// Defer to that same function so the app and core cannot disagree about where
/// projects live — this previously returned `<app_data_dir>/projects`, a
/// directory no code path ever creates, which made `delete_project` fail with
/// "Project not found" for every project.
///
/// Falls back to the app data dir only if the core lookup fails (e.g. no home
/// directory can be determined), preserving the previous behaviour as a last
/// resort.
pub fn get_projects_dir(app: &tauri::AppHandle) -> PathBuf {
    Project::projects_dir().unwrap_or_else(|_| get_app_data_dir(app).join("projects"))
}

/// Get the definitions directory (cross-platform).
pub fn get_definitions_dir(app: &tauri::AppHandle) -> PathBuf {
    get_app_data_dir(app).join("definitions")
}

/// Get the settings file path (cross-platform).
pub fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    get_app_data_dir(app).join("settings.json")
}

/// Get the dashboards directory (cross-platform).
pub fn get_dashboards_dir(app: &tauri::AppHandle) -> PathBuf {
    get_app_data_dir(app).join("dashboards")
}

/// Project-relative path to the port editor JSON store.
pub fn get_port_editor_store_path(project: &Project) -> PathBuf {
    project.path.join("projectCfg").join("port_editor.json")
}
