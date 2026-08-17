//! User-defined math channel Tauri commands.
//!
//! Math channels evaluate runtime expressions over output channels and
//! constants, exposing the result as a virtual channel for gauges and logs.

use crate::state::AppState;
use libretune_core::project::{save_math_channels, UserMathChannel};

#[tauri::command]
pub async fn get_math_channels(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<UserMathChannel>, String> {
    Ok(state.math_channels.lock().await.clone())
}

#[tauri::command]
pub async fn set_math_channel(
    state: tauri::State<'_, AppState>,
    mut channel: UserMathChannel,
) -> Result<(), String> {
    channel
        .compile()
        .map_err(|e| format!("Invalid expression: {}", e))?;

    let mut channels = state.math_channels.lock().await;

    if let Some(existing) = channels.iter_mut().find(|c| c.name == channel.name) {
        *existing = channel;
    } else {
        channels.push(channel);
    }

    let project = state.current_project.lock().await;
    if let Some(ref proj) = *project {
        let path = proj.path.join("math_channels.json");
        save_math_channels(&path, &channels)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_math_channel(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let mut channels = state.math_channels.lock().await;
    let initial_len = channels.len();
    channels.retain(|c| c.name != name);

    if channels.len() == initial_len {
        return Err(format!("Channel '{}' not found", name));
    }

    let project = state.current_project.lock().await;
    if let Some(ref proj) = *project {
        let path = proj.path.join("math_channels.json");
        save_math_channels(&path, &channels)?;
    }

    Ok(())
}

#[tauri::command]
pub async fn validate_math_expression(expr: String) -> Result<String, String> {
    let mut parser = libretune_core::ini::expression::Parser::new(&expr);
    match parser.parse() {
        Ok(_) => Ok("Valid expression".to_string()),
        Err(e) => Err(e),
    }
}
