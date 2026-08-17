//! Tune annotation Tauri commands.
//!
//! Annotations are user-authored notes attached to constants, tables, or
//! individual cells. Persisted in the .msq file alongside tune data.

use crate::state::AppState;
use libretune_core::tune::{AnnotationTag, TuneAnnotation};

/// Set an annotation on a constant, table, or cell.
/// Key format: `"constant_name"` for scalars, `"table_name:row:col"` for cells.
#[tauri::command]
pub async fn set_annotation(
    state: tauri::State<'_, AppState>,
    key: String,
    text: String,
    tag: Option<String>,
) -> Result<(), String> {
    let annotation_tag = tag.and_then(|t| match t.as_str() {
        "info" => Some(AnnotationTag::Info),
        "warning" => Some(AnnotationTag::Warning),
        "critical" => Some(AnnotationTag::Critical),
        "success" => Some(AnnotationTag::Success),
        "todo" => Some(AnnotationTag::Todo),
        _ => None,
    });

    let annotation = TuneAnnotation {
        text,
        author: None,
        created: chrono::Utc::now().to_rfc3339(),
        modified: None,
        tag: annotation_tag,
    };

    let mut tune_guard = state.current_tune.lock().await;
    let tune = tune_guard.as_mut().ok_or("No tune loaded")?;
    tune.set_annotation(key, annotation);

    Ok(())
}

#[tauri::command]
pub async fn get_annotation(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<Option<TuneAnnotation>, String> {
    let tune_guard = state.current_tune.lock().await;
    let tune = tune_guard.as_ref().ok_or("No tune loaded")?;
    Ok(tune.get_annotation(&key).cloned())
}

#[tauri::command]
pub async fn get_table_annotations(
    state: tauri::State<'_, AppState>,
    table_name: String,
) -> Result<Vec<(String, TuneAnnotation)>, String> {
    let tune_guard = state.current_tune.lock().await;
    let tune = tune_guard.as_ref().ok_or("No tune loaded")?;
    let annotations = tune
        .get_table_annotations(&table_name)
        .into_iter()
        .map(|(k, a)| (k.clone(), a.clone()))
        .collect();
    Ok(annotations)
}

#[tauri::command]
pub async fn delete_annotation(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<bool, String> {
    let mut tune_guard = state.current_tune.lock().await;
    let tune = tune_guard.as_mut().ok_or("No tune loaded")?;
    Ok(tune.delete_annotation(&key))
}

#[tauri::command]
pub async fn get_all_annotations(
    state: tauri::State<'_, AppState>,
) -> Result<std::collections::HashMap<String, TuneAnnotation>, String> {
    let tune_guard = state.current_tune.lock().await;
    let tune = tune_guard.as_ref().ok_or("No tune loaded")?;
    Ok(tune.all_annotations().clone())
}
