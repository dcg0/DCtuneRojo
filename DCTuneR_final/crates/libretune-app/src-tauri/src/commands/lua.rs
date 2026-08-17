//! Lua scripting Tauri commands.

use libretune_core::lua::{execute_script, LuaExecutionResult};

/// Execute a Lua script in the sandboxed runtime.
#[tauri::command]
pub async fn run_lua_script(script: String) -> Result<LuaExecutionResult, String> {
    execute_script(&script)
}
