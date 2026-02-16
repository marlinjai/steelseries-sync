mod backup;
mod config;
mod providers;
mod safety;
mod sync_engine;
pub mod tray;
mod watcher;

use config::{AppConfig, ProviderConfig, load_config, save_config_to_disk};
use providers::folder::FolderProvider;
use providers::hosted::HostedProvider;
use providers::SyncProvider;
use std::sync::Arc;
use sync_engine::SyncEngine;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;
use watcher::ConfigWatcher;

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
        Ok(result) => Ok(format_sync_result(&result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn push_now(state: State<'_, AppState>) -> Result<String, String> {
    match state.engine.push_to_remote().await {
        Ok(result) => Ok(format_sync_result(&result)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn pull_now(state: State<'_, AppState>) -> Result<String, String> {
    match state.engine.pull_from_remote().await {
        Ok(result) => Ok(format_sync_result(&result)),
        Err(e) => Err(e.to_string()),
    }
}

fn format_sync_result(result: &sync_engine::SyncResult) -> String {
    match result {
        sync_engine::SyncResult::Pushed => "Pushed".to_string(),
        sync_engine::SyncResult::Pulled { from_device, gg_was_running } => {
            if *gg_was_running {
                format!("Pulled from {}. Restart SteelSeries GG to apply changes.", from_device)
            } else {
                format!("Pulled from {}", from_device)
            }
        }
        sync_engine::SyncResult::Skipped(reason) => format!("Skipped({:?})", reason),
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
async fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    save_config_to_disk(&config).map_err(|e| e.to_string())?;
    let mut current = state.config.lock().await;
    *current = config;
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
    let config = load_config();
    let provider = build_provider(&config);
    let engine = Arc::new(SyncEngine::new(config.clone(), provider));

    let watcher_config_dir = config.steelseries_db_path.clone();
    let watcher_debounce = config.debounce_secs;

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
        .setup(move |app| {
            // Set up system tray
            let _ = tray::setup_tray(app.handle());

            // Spawn file watcher thread (outbound: local changes -> push)
            let watcher_handle = app.handle().clone();
            let watcher_engine = app.state::<AppState>().engine.clone();

            std::thread::spawn(move || {
                let watcher = ConfigWatcher::new(watcher_config_dir, watcher_debounce);
                let _ = watcher.watch(move |changed| {
                    log::info!("Config change detected at {:?}", changed.timestamp);

                    // Skip auto-push if we just pulled (prevents feedback loop)
                    if watcher_engine.should_suppress_push() {
                        log::info!("Suppressing auto-push after pull");
                        return;
                    }

                    let _ = watcher_handle.emit("sync-status", "syncing");
                    let engine = watcher_engine.clone();
                    let handle = watcher_handle.clone();
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    if let Ok(rt) = rt {
                        let result = rt.block_on(engine.push_to_remote());
                        match result {
                            Ok(r) => {
                                log::info!("Auto-push result: {:?}", r);
                                let _ = handle.emit("sync-status", format!("{:?}", r));
                            }
                            Err(e) => {
                                log::error!("Auto-push error: {}", e);
                                let _ = handle.emit("sync-status", format!("error: {}", e));
                            }
                        }
                    }
                });
            });

            // Spawn inbound sync polling (remote changes -> pull every 30s)
            let poll_handle = app.handle().clone();
            let poll_engine = app.state::<AppState>().engine.clone();

            tauri::async_runtime::spawn(async move {
                let mut last_seen = chrono::DateTime::<chrono::Utc>::MIN_UTC;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    log::info!("Polling remote for inbound changes...");

                    // Check if remote has new data we haven't seen
                    let meta = match poll_engine.remote_meta().await {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    if meta.last_modified <= last_seen {
                        log::debug!("Inbound poll: remote unchanged");
                        continue;
                    }

                    // Remote has newer data — pull it
                    match poll_engine.pull_from_remote().await {
                        Ok(ref r @ sync_engine::SyncResult::Pulled { ref from_device, .. }) => {
                            log::info!("Inbound sync: pulled from {}", from_device);
                            last_seen = meta.last_modified;
                            let _ = poll_handle.emit("sync-status", format_sync_result(r));
                        }
                        Ok(sync_engine::SyncResult::Skipped(ref reason)) => {
                            log::debug!("Inbound poll skipped: {:?}", reason);
                            last_seen = meta.last_modified;
                        }
                        Ok(_) => { last_seen = meta.last_modified; }
                        Err(e) => {
                            log::error!("Inbound poll error: {}", e);
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
