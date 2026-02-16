mod backup;
mod config;
mod providers;
mod safety;
mod sync_engine;
pub mod tray;
mod watcher;

use config::{AppConfig, ProviderConfig};
use providers::folder::FolderProvider;
use providers::hosted::HostedProvider;
use providers::SyncProvider;
use std::sync::Arc;
use sync_engine::SyncEngine;
use tauri::State;
use tokio::sync::Mutex;

struct AppState {
    engine: Arc<SyncEngine>,
    config: Mutex<AppConfig>,
}

fn build_provider(config: &AppConfig) -> Arc<dyn SyncProvider> {
    match &config.provider {
        ProviderConfig::Folder { sync_dir } => {
            Arc::new(FolderProvider::new(sync_dir.clone(), config.device_name.clone()))
        }
        ProviderConfig::Hosted { api_url, api_key } => {
            Arc::new(HostedProvider::new(
                api_url.clone(),
                api_key.clone(),
                config.device_name.clone(),
            ))
        }
    }
}

#[tauri::command]
async fn sync_now(state: State<'_, AppState>) -> Result<String, String> {
    match state.engine.sync().await {
        Ok(result) => Ok(format!("{:?}", result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn push_now(state: State<'_, AppState>) -> Result<String, String> {
    match state.engine.push_to_remote().await {
        Ok(result) => Ok(format!("{:?}", result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn pull_now(state: State<'_, AppState>) -> Result<String, String> {
    match state.engine.pull_from_remote().await {
        Ok(result) => Ok(format!("{:?}", result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn list_backups(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .engine
        .backups()
        .list_backups()
        .map(|entries| entries.iter().map(|e| e.name.clone()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<String, String> {
    let config = state.config.lock().await;
    serde_json::to_string(&*config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(state: State<'_, AppState>, config_json: String) -> Result<(), String> {
    let new_config: AppConfig =
        serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
    let mut config = state.config.lock().await;
    *config = new_config;
    Ok(())
}

#[tauri::command]
async fn restore_backup(
    state: State<'_, AppState>,
    backup_name: String,
) -> Result<String, String> {
    let config = state.config.lock().await;
    let backup_dir = config.backup_dir.join(&backup_name);
    if !backup_dir.exists() {
        return Err(format!("Backup '{}' not found", backup_name));
    }
    state
        .engine
        .backups()
        .restore_backup(&backup_dir, &config.steelseries_db_path)
        .map_err(|e| e.to_string())?;
    Ok(format!("Restored backup '{}'", backup_name))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = AppConfig::default();
    let provider = build_provider(&config);
    let engine = Arc::new(SyncEngine::new(config.clone(), provider));

    let app_state = AppState {
        engine,
        config: Mutex::new(config),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            sync_now,
            push_now,
            pull_now,
            list_backups,
            get_config,
            save_config,
            restore_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
