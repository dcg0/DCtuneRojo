//! Miscellaneous project commands: MSQ info preview, project deletion.

use crate::paths::get_projects_dir;
use crate::state::AppState;
use libretune_core::project::Project;
use libretune_core::tune::TuneFile;

/// Get info about an MSQ file without fully loading it (for the open dialog preview)
#[tauri::command]
pub async fn get_msq_info(path: String) -> Result<serde_json::Value, String> {
    let file_path = std::path::Path::new(&path);
    if !file_path.exists() {
        return Err("File not found".to_string());
    }

    let tune = TuneFile::load(file_path).map_err(|e| format!("Failed to read MSQ: {}", e))?;

    let mut info = serde_json::Map::new();
    info.insert(
        "signature".to_string(),
        serde_json::Value::String(tune.signature.clone()),
    );
    info.insert(
        "version".to_string(),
        serde_json::Value::String(tune.version.clone()),
    );
    info.insert(
        "file_name".to_string(),
        serde_json::Value::String(
            file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        ),
    );
    info.insert(
        "file_size".to_string(),
        serde_json::Value::Number(serde_json::Number::from(
            std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0),
        )),
    );

    // Count constants
    let constant_count = tune.constants.len();
    info.insert(
        "constant_count".to_string(),
        serde_json::Value::Number(serde_json::Number::from(constant_count)),
    );

    // INI metadata if present
    if let Some(ref meta) = tune.ini_metadata {
        info.insert(
            "ini_name".to_string(),
            serde_json::Value::String(meta.name.clone()),
        );
        info.insert(
            "saved_at".to_string(),
            serde_json::Value::String(meta.saved_at.clone()),
        );
    }

    // Author and description
    if let Some(ref author) = tune.author {
        info.insert(
            "author".to_string(),
            serde_json::Value::String(author.to_string()),
        );
    }
    if let Some(ref desc) = tune.description {
        info.insert(
            "description".to_string(),
            serde_json::Value::String(desc.to_string()),
        );
    }

    Ok(serde_json::Value::Object(info))
}

/// Delete a project and all its files
#[tauri::command]
pub async fn delete_project(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    project_name: String,
) -> Result<(), String> {
    // Resolve the project by the name it records, not by assuming its directory
    // is named after it. `Project::create` sanitises the display name for the
    // filesystem, and a project directory can be copied or renamed on disk, so
    // the folder name and `project.json`'s `name` are not always equal.
    // `list_projects` already reports both, and the UI lists projects from it,
    // so resolving through it deletes exactly what the user selected.
    //
    // Fall back to the old name-joined path if the project is not in the list
    // (e.g. an unreadable `project.json`), preserving the previous behaviour.
    if project_name.trim().is_empty()
        || !std::path::Path::new(&project_name)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err("Invalid project name".to_string());
    }

    let project_path = match Project::list_projects() {
        Ok(projects) => projects
            .into_iter()
            .find(|p| p.name == project_name)
            .map(|p| p.path)
            .unwrap_or_else(|| get_projects_dir(&app).join(&project_name)),
        Err(_) => get_projects_dir(&app).join(&project_name),
    };

    if !project_path.exists() {
        return Err(format!("Project '{}' not found", project_name));
    }

    // Don't allow deleting the currently open project
    let proj_guard = state.current_project.lock().await;
    if let Some(ref proj) = *proj_guard {
        if proj.config.name == project_name {
            return Err("Cannot delete the currently open project. Close it first.".to_string());
        }
    }
    drop(proj_guard);

    std::fs::remove_dir_all(&project_path)
        .map_err(|e| format!("Failed to delete project: {}", e))?;

    Ok(())
}
