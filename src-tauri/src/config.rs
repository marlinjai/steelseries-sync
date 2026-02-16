//! Application configuration — paths, provider settings, sync options.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Path to SteelSeries GG config directory
    pub steelseries_db_path: PathBuf,
    /// Path to local backup directory
    pub backup_dir: PathBuf,
    /// Maximum number of backups to retain
    pub max_backups: usize,
    /// Debounce duration in seconds
    pub debounce_secs: u64,
    /// Active sync provider
    pub provider: ProviderConfig,
    /// Device name for this machine (used in conflict labels)
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderConfig {
    Folder { sync_dir: PathBuf },
    Hosted { api_url: String, api_key: String },
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            steelseries_db_path: default_steelseries_path(),
            backup_dir: default_backup_path(),
            max_backups: 20,
            debounce_secs: 3,
            provider: ProviderConfig::Folder {
                sync_dir: default_sync_folder(),
            },
            device_name: hostname(),
        }
    }
}

fn default_steelseries_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\ProgramData\SteelSeries\SteelSeries Engine 3\db")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/SteelSeries Engine 3/db")
    } else {
        PathBuf::from("/etc/steelseries-engine-3/db")
    }
}

fn default_backup_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("steelseries-sync")
        .join("backups")
}

fn default_sync_folder() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("SteelSeriesSync")
}

fn hostname() -> String {
    sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string())
}
