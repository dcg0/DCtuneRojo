//! Hotkey bindings and onboarding state Tauri commands.

use std::collections::HashMap;
use tauri::Emitter;

use crate::{load_settings, save_settings};

/// Get current hotkey bindings from settings.
#[tauri::command]
pub async fn get_hotkey_bindings(app: tauri::AppHandle) -> Result<HashMap<String, String>, String> {
    let settings = load_settings(&app);
    Ok(settings.hotkey_bindings.clone())
}

/// Save hotkey bindings to settings.
#[tauri::command]
pub async fn save_hotkey_bindings(
    app: tauri::AppHandle,
    bindings: HashMap<String, String>,
) -> Result<(), String> {
    let mut settings = load_settings(&app);
    settings.hotkey_bindings = bindings;
    save_settings(&app, &settings);
    let _ = app.emit("settings:hotkeys_changed", ());
    Ok(())
}

/// Mark onboarding as completed.
#[tauri::command]
pub async fn mark_onboarding_completed(app: tauri::AppHandle) -> Result<(), String> {
    let mut settings = load_settings(&app);
    settings.onboarding_completed = true;
    save_settings(&app, &settings);
    Ok(())
}

/// Check if onboarding has been completed.
#[tauri::command]
pub async fn is_onboarding_completed(app: tauri::AppHandle) -> Result<bool, String> {
    let settings = load_settings(&app);
    Ok(settings.onboarding_completed)
}
