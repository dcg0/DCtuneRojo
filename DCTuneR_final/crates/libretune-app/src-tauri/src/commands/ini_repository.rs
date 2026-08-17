//! Local INI repository Tauri commands.

use libretune_core::project::IniRepository;
use serde::Serialize;
use std::path::Path;

use crate::state::AppState;

#[derive(Serialize)]
pub struct IniEntryResponse {
    id: String,
    name: String,
    signature: String,
    path: String,
}

/// Initialize the INI repository for managing ECU definition files.
#[tauri::command]
pub async fn init_ini_repository(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let repo =
        IniRepository::open(None).map_err(|e| format!("Failed to open INI repository: {}", e))?;

    let path = repo.path.to_string_lossy().to_string();

    let mut guard = state.ini_repository.lock().await;
    *guard = Some(repo);

    Ok(path)
}

/// List INIs in the repository
#[tauri::command]
pub async fn list_repository_inis(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<IniEntryResponse>, String> {
    let guard = state.ini_repository.lock().await;
    let repo = guard
        .as_ref()
        .ok_or_else(|| "INI repository not initialized".to_string())?;

    Ok(repo
        .list()
        .iter()
        .map(|e| IniEntryResponse {
            id: e.id.clone(),
            name: e.name.clone(),
            signature: e.signature.clone(),
            path: e.path.clone(),
        })
        .collect())
}

/// Import an INI file into the local repository.
#[tauri::command]
pub async fn import_ini(
    state: tauri::State<'_, AppState>,
    source_path: String,
) -> Result<IniEntryResponse, String> {
    let mut guard = state.ini_repository.lock().await;
    let repo = guard
        .as_mut()
        .ok_or_else(|| "INI repository not initialized".to_string())?;

    let id = repo
        .import(Path::new(&source_path))
        .map_err(|e| format!("Failed to import INI: {}", e))?;

    let entry = repo
        .get(&id)
        .ok_or_else(|| "Failed to get imported INI".to_string())?;

    Ok(IniEntryResponse {
        id: entry.id.clone(),
        name: entry.name.clone(),
        signature: entry.signature.clone(),
        path: entry.path.clone(),
    })
}

/// Scan a directory for INI files and import them all.
#[tauri::command]
pub async fn scan_for_inis(
    state: tauri::State<'_, AppState>,
    directory: String,
) -> Result<Vec<String>, String> {
    let mut guard = state.ini_repository.lock().await;
    let repo = guard
        .as_mut()
        .ok_or_else(|| "INI repository not initialized".to_string())?;

    repo.scan_directory(Path::new(&directory))
        .map_err(|e| format!("Failed to scan directory: {}", e))
}

/// Remove an INI file from the repository.
#[tauri::command]
pub async fn remove_ini(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let mut guard = state.ini_repository.lock().await;
    let repo = guard
        .as_mut()
        .ok_or_else(|| "INI repository not initialized".to_string())?;

    repo.remove(&id)
        .map_err(|e| format!("Failed to remove INI: {}", e))
}
