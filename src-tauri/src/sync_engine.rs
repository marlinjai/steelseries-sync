//! Core sync orchestration — coordinates watcher, safety, backup, and provider.

use crate::backup::BackupManager;
use crate::config::AppConfig;
use crate::providers::{ConfigSnapshot, ProviderError, SyncMeta, SyncProvider};
use crate::safety::{validate_sqlite_header, SafetyCheck, SafetyGuard};
use chrono::Utc;
use std::fs;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SyncEngine {
    config: AppConfig,
    provider: Arc<dyn SyncProvider>,
    backup_manager: BackupManager,
    safety: Mutex<SafetyGuard>,
    /// Suppresses the next watcher-triggered push after a pull (prevents feedback loop).
    pull_in_progress: std::sync::atomic::AtomicBool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncResult {
    Pushed,
    Pulled { from_device: String, gg_was_running: bool },
    Skipped(SkipReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SkipReason {
    GGRunning,
    FileLocked,
    NoLocalConfig,
    NoRemoteConfig,
    AlreadyInSync,
    InvalidRemoteFile,
}

impl SyncEngine {
    pub fn new(config: AppConfig, provider: Arc<dyn SyncProvider>) -> Self {
        let backup_manager = BackupManager::new(
            config.backup_dir.clone(),
            config.max_backups,
        );
        Self {
            config,
            provider,
            backup_manager,
            safety: Mutex::new(SafetyGuard::new()),
            pull_in_progress: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Read local config files into a snapshot.
    fn read_local_config(&self) -> std::io::Result<ConfigSnapshot> {
        let dir = &self.config.steelseries_db_path;
        let db = fs::read(dir.join("database.db"))?;
        let db_shm = fs::read(dir.join("database.db-shm")).ok();
        let db_wal = fs::read(dir.join("database.db-wal")).ok();
        Ok(ConfigSnapshot {
            db,
            db_shm,
            db_wal,
            meta: SyncMeta {
                last_modified: Utc::now(),
                device_name: self.config.device_name.clone(),
            },
        })
    }

    /// Write a snapshot to the local config directory.
    fn write_local_config(&self, snapshot: &ConfigSnapshot) -> std::io::Result<()> {
        let dir = &self.config.steelseries_db_path;
        fs::create_dir_all(dir)?;
        fs::write(dir.join("database.db"), &snapshot.db)?;
        if let Some(shm) = &snapshot.db_shm {
            fs::write(dir.join("database.db-shm"), shm)?;
        }
        if let Some(wal) = &snapshot.db_wal {
            fs::write(dir.join("database.db-wal"), wal)?;
        }
        Ok(())
    }

    /// Push local config to the remote provider.
    pub async fn push_to_remote(&self) -> Result<SyncResult, SyncError> {
        let mut safety = self.safety.lock().await;
        match safety.is_safe_to_read(&self.config.steelseries_db_path) {
            SafetyCheck::Safe => {}
            SafetyCheck::NoConfig => return Ok(SyncResult::Skipped(SkipReason::NoLocalConfig)),
            SafetyCheck::FileLocked => return Ok(SyncResult::Skipped(SkipReason::FileLocked)),
            SafetyCheck::GGRunning => {} // safe to read while GG runs
        }
        drop(safety);

        let snapshot = self.read_local_config()?;
        self.provider.push(&snapshot).await?;
        Ok(SyncResult::Pushed)
    }

    /// Pull remote config and overwrite local (with backup).
    pub async fn pull_from_remote(&self) -> Result<SyncResult, SyncError> {
        let mut safety = self.safety.lock().await;
        let gg_was_running = safety.is_gg_running();
        match safety.is_safe_to_read(&self.config.steelseries_db_path) {
            SafetyCheck::Safe => {}
            SafetyCheck::FileLocked => return Ok(SyncResult::Skipped(SkipReason::FileLocked)),
            SafetyCheck::NoConfig => {} // OK to write even if no existing config
            SafetyCheck::GGRunning => {} // unreachable from is_safe_to_read
        }
        drop(safety);

        let remote = match self.provider.pull().await {
            Ok(r) => r,
            Err(ProviderError::NotFound) => {
                return Ok(SyncResult::Skipped(SkipReason::NoRemoteConfig))
            }
            Err(e) => return Err(e.into()),
        };

        // Validate remote data
        if !validate_sqlite_header(&remote.db) {
            return Ok(SyncResult::Skipped(SkipReason::InvalidRemoteFile));
        }

        // Backup current local before overwriting
        if self.config.steelseries_db_path.join("database.db").exists() {
            self.backup_manager
                .create_backup(&self.config.steelseries_db_path, "pre-pull")?;
        }

        // Suppress watcher auto-push for this write (prevents feedback loop)
        self.pull_in_progress.store(true, std::sync::atomic::Ordering::SeqCst);
        self.write_local_config(&remote)?;
        Ok(SyncResult::Pulled {
            from_device: remote.meta.device_name,
            gg_was_running,
        })
    }

    /// Full sync: compare timestamps, push or pull as needed.
    pub async fn sync(&self) -> Result<SyncResult, SyncError> {
        let local_exists = self.config.steelseries_db_path.join("database.db").exists();

        let remote_meta = match self.provider.remote_meta().await {
            Ok(m) => Some(m),
            Err(ProviderError::NotFound) => None,
            Err(e) => return Err(e.into()),
        };

        match (local_exists, remote_meta) {
            // Both exist: compare timestamps (last-write-wins)
            (true, Some(remote)) => {
                let local_modified = fs::metadata(
                    self.config.steelseries_db_path.join("database.db"),
                )?
                .modified()?;
                let local_ts = chrono::DateTime::<Utc>::from(local_modified);

                if local_ts > remote.last_modified {
                    // Local is newer -- push
                    self.backup_manager
                        .create_backup(&self.config.steelseries_db_path, "pre-push")?;
                    self.push_to_remote().await
                } else if remote.last_modified > local_ts {
                    // Remote is newer -- pull
                    self.pull_from_remote().await
                } else {
                    Ok(SyncResult::Skipped(SkipReason::AlreadyInSync))
                }
            }
            // Only local exists -- push
            (true, None) => self.push_to_remote().await,
            // Only remote exists -- pull
            (false, Some(_)) => self.pull_from_remote().await,
            // Neither exists
            (false, None) => Ok(SyncResult::Skipped(SkipReason::NoLocalConfig)),
        }
    }

    /// Get remote metadata (for polling).
    pub async fn remote_meta(&self) -> Result<SyncMeta, SyncError> {
        self.provider.remote_meta().await.map_err(SyncError::from)
    }

    /// Check if a pull just happened (and reset the flag).
    /// The watcher should call this before auto-pushing to avoid feedback loops.
    pub fn should_suppress_push(&self) -> bool {
        self.pull_in_progress.swap(false, std::sync::atomic::Ordering::SeqCst)
    }

    /// Get a reference to the backup manager (for UI).
    pub fn backups(&self) -> &BackupManager {
        &self.backup_manager
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Provider error: {0}")]
    Provider(#[from] crate::providers::ProviderError),
}
