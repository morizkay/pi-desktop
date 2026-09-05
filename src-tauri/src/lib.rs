mod git;
mod preferences;
mod updates;
mod terminal;
mod pi_config;
mod pi_rpc;

use pi_config::{
    list_auth_providers, list_models_for_provider, list_provider_ids_from_store, patch_settings,
    pi_install, pi_list_packages, pi_remove, pi_update_all_extensions, pi_update_package,
    read_settings, set_api_key, PiAuthProviderView, PiCliResult, PiSettingsView,
};
use pi_rpc::{PiRpcManager, PiSessionSummary};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

type AppState = Arc<PiRpcManager>;

// Never hold application state or an async executor thread while waiting on RPC.
async fn call(app: AppHandle, pi: AppState, command: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let cwd = pi.get_cwd()?;
        pi.rpc_call(&app, &cwd, command)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn pi_list_sessions(project_cwd: Option<String>) -> Result<Vec<PiSessionSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        PiRpcManager::list_sessions(project_cwd.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn pi_get_cwd(state: State<'_, AppState>) -> Result<String, String> {
    state.get_cwd()
}
#[tauri::command]
fn pi_set_cwd(state: State<'_, AppState>, cwd: String) -> Result<(), String> {
    state.set_cwd(cwd)
}
#[tauri::command]
async fn pi_rpc(
    app: AppHandle,
    state: State<'_, AppState>,
    command: Value,
) -> Result<Value, String> {
    call(app, state.inner().clone(), command).await
}
#[tauri::command]
async fn pi_extension_response(state: State<'_, AppState>, response: Value) -> Result<(), String> {
    let pi = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || pi.send_extension_response(response))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn pi_new_session(
    app: AppHandle,
    state: State<'_, AppState>,
    cwd: Option<String>,
) -> Result<Value, String> {
    let pi = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || pi.new_session(&app, cwd))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn pi_switch_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session_path: String,
) -> Result<Value, String> {
    call(
        app,
        state.inner().clone(),
        json!({"type":"switch_session", "sessionPath":session_path}),
    )
    .await
}
#[tauri::command]
async fn pi_prompt(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
) -> Result<Value, String> {
    call(
        app,
        state.inner().clone(),
        json!({"type":"prompt", "message":message}),
    )
    .await
}
#[tauri::command]
async fn pi_abort(app: AppHandle, state: State<'_, AppState>) -> Result<Value, String> {
    call(app, state.inner().clone(), json!({"type":"abort"})).await
}
#[tauri::command]
async fn pi_get_messages(app: AppHandle, state: State<'_, AppState>) -> Result<Value, String> {
    call(app, state.inner().clone(), json!({"type":"get_messages"})).await
}
#[tauri::command]
async fn pi_get_state_rpc(app: AppHandle, state: State<'_, AppState>) -> Result<Value, String> {
    call(app, state.inner().clone(), json!({"type":"get_state"})).await
}
#[tauri::command]
async fn pi_get_available_models_rpc(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    call(
        app,
        state.inner().clone(),
        json!({"type":"get_available_models"}),
    )
    .await
}
#[tauri::command]
async fn pi_set_model_rpc(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
    model_id: String,
) -> Result<Value, String> {
    call(
        app,
        state.inner().clone(),
        json!({"type":"set_model", "provider":provider, "modelId":model_id}),
    )
    .await
}
#[tauri::command]
async fn pi_git_status(state: State<'_, AppState>) -> Result<git::GitStatus, String> {
    let cwd = state.get_cwd()?;
    tauri::async_runtime::spawn_blocking(move || git::status(&cwd))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn pi_git_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
) -> Result<String, String> {
    let cwd = state.get_cwd()?;
    tauri::async_runtime::spawn_blocking(move || git::diff(&cwd, &path, staged))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn pi_read_settings() -> Result<PiSettingsView, String> {
    read_settings()
}

#[tauri::command]
fn pi_patch_settings(
    default_provider: Option<String>,
    default_model: Option<String>,
    default_thinking_level: Option<String>,
) -> Result<PiSettingsView, String> {
    patch_settings(default_provider, default_model, default_thinking_level)
}

#[tauri::command]
fn pi_list_auth_providers() -> Result<Vec<PiAuthProviderView>, String> {
    list_auth_providers()
}

#[tauri::command]
fn pi_set_api_key(provider: String, key: String) -> Result<(), String> {
    set_api_key(&provider, &key)
}

#[tauri::command]
fn pi_package_install(source: String) -> PiCliResult {
    pi_install(&source)
}

#[tauri::command]
fn pi_package_remove(source: String) -> PiCliResult {
    pi_remove(&source)
}

#[tauri::command]
fn pi_package_update(source: String) -> PiCliResult {
    pi_update_package(&source)
}

#[tauri::command]
fn pi_package_update_all() -> PiCliResult {
    pi_update_all_extensions()
}

#[tauri::command]
fn pi_package_list_cli() -> PiCliResult {
    pi_list_packages()
}

#[tauri::command]
fn pi_list_provider_ids() -> Result<Vec<String>, String> {
    list_provider_ids_from_store()
}

#[tauri::command]
fn pi_list_models_store(provider: String) -> Result<Vec<Value>, String> {
    list_models_for_provider(&provider)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(Arc::new(PiRpcManager::new()));
            app.manage(Arc::new(terminal::Terminals::default()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            terminal::terminal_start,
            terminal::terminal_poll,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_close,
            preferences::pi_save_enabled_models,
            preferences::pi_remove_api_key,
            preferences::pi_custom_providers,
            preferences::pi_save_custom_provider,
            preferences::pi_remove_custom_provider,
            updates::pi_check_update,
            pi_list_sessions,
            pi_get_cwd,
            pi_set_cwd,
            pi_rpc,
            pi_extension_response,
            pi_git_status,
            pi_git_diff,
            pi_new_session,
            pi_switch_session,
            pi_prompt,
            pi_abort,
            pi_get_messages,
            pi_read_settings,
            pi_patch_settings,
            pi_list_auth_providers,
            pi_set_api_key,
            pi_package_install,
            pi_package_remove,
            pi_package_update,
            pi_package_update_all,
            pi_package_list_cli,
            pi_list_provider_ids,
            pi_list_models_store,
            pi_set_model_rpc,
            pi_get_available_models_rpc,
            pi_get_state_rpc,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                app.state::<Arc<terminal::Terminals>>().close_all();
                let _ = app.state::<AppState>().stop();
            }
        });
}
