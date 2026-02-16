# SteelSeries Sync — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an open-source Tauri desktop app that syncs SteelSeries GG config files across machines, with an optional hosted sync tier.

**Architecture:** File watcher monitors SteelSeries config dir, debounces changes, creates timestamped backups, and pushes to a sync provider (cloud folder or hosted API). Inbound sync pulls remote changes and overwrites local config after backup. Adapter pattern for providers.

**Tech Stack:** Tauri 2 (Rust + WebView), React + TypeScript + Vite, `notify`/`sysinfo`/`reqwest`/`chrono` crates, NestJS for hosted API, PM2 + Cloudflare Tunnel for deployment.

**Design doc:** `docs/plans/2026-02-16-steelseries-sync-design.md`

---

## Phase 1: Project Scaffold

### Task 1: Initialize Tauri 2 + React + TypeScript project

**Files:**
- Create: entire project scaffold via `create-tauri-app`
- Modify: `src-tauri/Cargo.toml` (add dependencies)
- Modify: `package.json` (verify scripts)

**Step 1: Scaffold the Tauri project**

Run:
```bash
cd "/Users/marlinjai/software dev/steelseries-sync"
npm create tauri-app@latest . -- --template react-ts --manager npm
```

If the directory isn't empty (we have docs/), move docs out, scaffold, move docs back.

**Step 2: Add Rust dependencies**

In `src-tauri/Cargo.toml`, add under `[dependencies]`:
```toml
notify = "7"
sysinfo = "0.33"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
dirs = "6"
log = "0.4"
env_logger = "0.11"
```

**Step 3: Verify the app builds and launches**

Run:
```bash
npm install
npm run tauri dev
```

Expected: Default Tauri window opens with React template.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri 2 + React + TypeScript project"
```

---

### Task 2: Create Rust module structure

**Files:**
- Create: `src-tauri/src/watcher.rs`
- Create: `src-tauri/src/sync_engine.rs`
- Create: `src-tauri/src/backup.rs`
- Create: `src-tauri/src/safety.rs`
- Create: `src-tauri/src/providers/mod.rs`
- Create: `src-tauri/src/providers/folder.rs`
- Create: `src-tauri/src/providers/hosted.rs`
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/main.rs` (or `lib.rs`)

**Step 1: Create empty module files with doc comments**

Each file should have a module-level doc comment explaining its purpose and be registered in `main.rs`/`lib.rs`.

`src-tauri/src/config.rs`:
```rust
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
```

`src-tauri/src/providers/mod.rs`:
```rust
//! Provider adapter interface and implementations.

pub mod folder;
pub mod hosted;

use std::path::PathBuf;

/// Metadata about a synced config set.
#[derive(Debug, Clone)]
pub struct SyncMeta {
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub device_name: String,
}

/// Files that make up a SteelSeries config snapshot.
#[derive(Debug, Clone)]
pub struct ConfigSnapshot {
    pub db: Vec<u8>,
    pub db_shm: Option<Vec<u8>>,
    pub db_wal: Option<Vec<u8>>,
    pub meta: SyncMeta,
}

/// The result type for provider operations.
pub type ProviderResult<T> = Result<T, ProviderError>;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("No remote config found")]
    NotFound,
    #[error("Provider error: {0}")]
    Other(String),
}

/// Trait that all sync providers implement.
#[async_trait::async_trait]
pub trait SyncProvider: Send + Sync {
    /// Push local config to the remote.
    async fn push(&self, snapshot: &ConfigSnapshot) -> ProviderResult<()>;

    /// Pull remote config.
    async fn pull(&self) -> ProviderResult<ConfigSnapshot>;

    /// Get metadata about the remote config without downloading files.
    async fn remote_meta(&self) -> ProviderResult<SyncMeta>;
}
```

Other files start as stubs:

`src-tauri/src/watcher.rs`:
```rust
//! File system watcher with debouncing for SteelSeries config changes.
```

`src-tauri/src/sync_engine.rs`:
```rust
//! Core sync orchestration — coordinates watcher, safety, backup, and provider.
```

`src-tauri/src/backup.rs`:
```rust
//! Timestamped backup manager with configurable retention.
```

`src-tauri/src/safety.rs`:
```rust
//! Safety guard — checks GG process state and file locks before sync operations.
```

`src-tauri/src/providers/folder.rs`:
```rust
//! Cloud folder sync provider (Dropbox, OneDrive, Google Drive, iCloud, etc.)
```

`src-tauri/src/providers/hosted.rs`:
```rust
//! Hosted API sync provider — communicates with the Mac Mini sync server.
```

**Step 2: Register modules in lib.rs**

Add to `src-tauri/src/lib.rs` (or `main.rs` depending on Tauri scaffold):
```rust
mod backup;
mod config;
mod providers;
mod safety;
mod sync_engine;
mod watcher;
```

Also add `thiserror` and `async-trait` to `Cargo.toml`:
```toml
thiserror = "2"
async-trait = "0.1"
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles with no errors (may have unused warnings, that's fine).

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: create Rust module structure with config and provider trait"
```

---

## Phase 2: Core Rust Backend

### Task 3: Implement Backup Manager

**Files:**
- Modify: `src-tauri/src/backup.rs`

**Step 1: Write failing tests**

```rust
//! Timestamped backup manager with configurable retention.

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

pub struct BackupManager {
    backup_dir: PathBuf,
    max_backups: usize,
}

impl BackupManager {
    pub fn new(backup_dir: PathBuf, max_backups: usize) -> Self {
        Self { backup_dir, max_backups }
    }

    /// Create a timestamped backup of the given files.
    /// Returns the path to the backup directory.
    pub fn create_backup(&self, source_dir: &Path, label: &str) -> std::io::Result<PathBuf> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S");
        let backup_name = format!("{}-{}", label, timestamp);
        let backup_path = self.backup_dir.join(&backup_name);
        fs::create_dir_all(&backup_path)?;

        // Copy all database.db* files
        for entry in fs::read_dir(source_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("database.db") {
                fs::copy(entry.path(), backup_path.join(&name))?;
            }
        }

        self.prune_old_backups()?;
        Ok(backup_path)
    }

    /// List all backups, newest first.
    pub fn list_backups(&self) -> std::io::Result<Vec<BackupEntry>> {
        let mut entries = Vec::new();
        if !self.backup_dir.exists() {
            return Ok(entries);
        }
        for entry in fs::read_dir(&self.backup_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                let modified = entry.metadata()?.modified()?;
                entries.push(BackupEntry {
                    name,
                    path: entry.path(),
                    created: modified,
                });
            }
        }
        entries.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(entries)
    }

    /// Restore a backup to the target directory.
    pub fn restore_backup(&self, backup_path: &Path, target_dir: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(backup_path)? {
            let entry = entry?;
            let name = entry.file_name();
            fs::copy(entry.path(), target_dir.join(&name))?;
        }
        Ok(())
    }

    fn prune_old_backups(&self) -> std::io::Result<()> {
        let backups = self.list_backups()?;
        if backups.len() > self.max_backups {
            for old in &backups[self.max_backups..] {
                fs::remove_dir_all(&old.path)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BackupEntry {
    pub name: String,
    pub path: PathBuf,
    pub created: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_source_dir(tmp: &TempDir) -> PathBuf {
        let src = tmp.path().join("source");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("database.db"), b"main-db-content").unwrap();
        fs::write(src.join("database.db-shm"), b"shm-content").unwrap();
        fs::write(src.join("database.db-wal"), b"wal-content").unwrap();
        fs::write(src.join("unrelated.txt"), b"ignore-me").unwrap();
        src
    }

    #[test]
    fn test_create_backup_copies_only_db_files() {
        let tmp = TempDir::new().unwrap();
        let src = setup_source_dir(&tmp);
        let backup_dir = tmp.path().join("backups");
        let mgr = BackupManager::new(backup_dir, 20);

        let backup_path = mgr.create_backup(&src, "sync").unwrap();

        assert!(backup_path.join("database.db").exists());
        assert!(backup_path.join("database.db-shm").exists());
        assert!(backup_path.join("database.db-wal").exists());
        assert!(!backup_path.join("unrelated.txt").exists());
    }

    #[test]
    fn test_list_backups_returns_newest_first() {
        let tmp = TempDir::new().unwrap();
        let src = setup_source_dir(&tmp);
        let backup_dir = tmp.path().join("backups");
        let mgr = BackupManager::new(backup_dir, 20);

        mgr.create_backup(&src, "first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        mgr.create_backup(&src, "second").unwrap();

        let list = mgr.list_backups().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].name.starts_with("second"));
        assert!(list[1].name.starts_with("first"));
    }

    #[test]
    fn test_prune_respects_max_backups() {
        let tmp = TempDir::new().unwrap();
        let src = setup_source_dir(&tmp);
        let backup_dir = tmp.path().join("backups");
        let mgr = BackupManager::new(backup_dir, 2);

        mgr.create_backup(&src, "one").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        mgr.create_backup(&src, "two").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        mgr.create_backup(&src, "three").unwrap();

        let list = mgr.list_backups().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].name.starts_with("three"));
        assert!(list[1].name.starts_with("two"));
    }

    #[test]
    fn test_restore_backup() {
        let tmp = TempDir::new().unwrap();
        let src = setup_source_dir(&tmp);
        let backup_dir = tmp.path().join("backups");
        let mgr = BackupManager::new(backup_dir, 20);

        let backup_path = mgr.create_backup(&src, "snap").unwrap();

        // Modify the source
        fs::write(src.join("database.db"), b"modified").unwrap();

        // Restore
        mgr.restore_backup(&backup_path, &src).unwrap();
        assert_eq!(fs::read(src.join("database.db")).unwrap(), b"main-db-content");
    }
}
```

Also add `tempfile` to dev-dependencies in `Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test backup`
Expected: All 4 tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement backup manager with timestamped backups and retention"
```

---

### Task 4: Implement Safety Guard

**Files:**
- Modify: `src-tauri/src/safety.rs`

**Step 1: Write implementation with tests**

```rust
//! Safety guard — checks GG process state and file locks before sync operations.

use std::path::Path;
use sysinfo::System;

const GG_PROCESS_NAMES: &[&str] = &[
    "SteelSeriesGG",
    "SteelSeriesGG.exe",
    "SteelSeriesEngine",
    "SteelSeriesEngine.exe",
    "SteelSeriesEngine3",
    "SteelSeriesEngine3.exe",
];

pub struct SafetyGuard {
    system: System,
}

impl SafetyGuard {
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }

    /// Check if SteelSeries GG is currently running.
    pub fn is_gg_running(&mut self) -> bool {
        self.system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.system.processes().values().any(|p| {
            let name = p.name().to_string_lossy();
            GG_PROCESS_NAMES.iter().any(|gg| name.contains(gg))
        })
    }

    /// Check if a file can be read (not locked by another process).
    pub fn can_read_file(path: &Path) -> bool {
        std::fs::File::open(path).is_ok()
    }

    /// Check if the config directory is safe to read from.
    pub fn is_safe_to_read(&mut self, config_dir: &Path) -> SafetyCheck {
        let db_path = config_dir.join("database.db");
        if !db_path.exists() {
            return SafetyCheck::NoConfig;
        }
        if !Self::can_read_file(&db_path) {
            return SafetyCheck::FileLocked;
        }
        SafetyCheck::Safe
    }

    /// Check if the config directory is safe to write to.
    /// Writing while GG is running can corrupt the database.
    pub fn is_safe_to_write(&mut self, config_dir: &Path) -> SafetyCheck {
        if self.is_gg_running() {
            return SafetyCheck::GGRunning;
        }
        let db_path = config_dir.join("database.db");
        if !db_path.exists() {
            return SafetyCheck::NoConfig;
        }
        if !Self::can_read_file(&db_path) {
            return SafetyCheck::FileLocked;
        }
        SafetyCheck::Safe
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyCheck {
    Safe,
    GGRunning,
    FileLocked,
    NoConfig,
}

/// Validate that a file looks like a valid SQLite database.
/// Checks the SQLite magic header bytes.
pub fn validate_sqlite_header(data: &[u8]) -> bool {
    const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";
    data.len() > SQLITE_MAGIC.len() && data[..SQLITE_MAGIC.len()] == *SQLITE_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_sqlite_header_valid() {
        let mut data = b"SQLite format 3\0".to_vec();
        data.extend_from_slice(&[0u8; 100]);
        assert!(validate_sqlite_header(&data));
    }

    #[test]
    fn test_validate_sqlite_header_invalid() {
        assert!(!validate_sqlite_header(b"not a database"));
        assert!(!validate_sqlite_header(b""));
        assert!(!validate_sqlite_header(b"SQLite format 3")); // missing null
    }

    #[test]
    fn test_safety_check_no_config() {
        let tmp = TempDir::new().unwrap();
        let mut guard = SafetyGuard::new();
        assert_eq!(guard.is_safe_to_read(tmp.path()), SafetyCheck::NoConfig);
    }

    #[test]
    fn test_safety_check_safe_to_read() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("database.db"), b"test").unwrap();
        let mut guard = SafetyGuard::new();
        assert_eq!(guard.is_safe_to_read(tmp.path()), SafetyCheck::Safe);
    }

    #[test]
    fn test_can_read_existing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        std::fs::write(&path, b"data").unwrap();
        assert!(SafetyGuard::can_read_file(&path));
    }

    #[test]
    fn test_cannot_read_missing_file() {
        assert!(!SafetyGuard::can_read_file(Path::new("/nonexistent/file.db")));
    }
}
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test safety`
Expected: All tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement safety guard with GG process detection and SQLite validation"
```

---

### Task 5: Implement File Watcher with Debouncer

**Files:**
- Modify: `src-tauri/src/watcher.rs`

**Step 1: Write implementation with tests**

```rust
//! File system watcher with debouncing for SteelSeries config changes.

use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct ConfigWatcher {
    config_dir: PathBuf,
    debounce_duration: Duration,
}

/// Event emitted when a debounced config change is detected.
#[derive(Debug, Clone)]
pub struct ConfigChanged {
    pub config_dir: PathBuf,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ConfigWatcher {
    pub fn new(config_dir: PathBuf, debounce_secs: u64) -> Self {
        Self {
            config_dir,
            debounce_duration: Duration::from_secs(debounce_secs),
        }
    }

    /// Start watching. Calls `on_change` after debounced changes to database.db* files.
    /// This blocks the current thread. Run in a dedicated thread.
    pub fn watch<F>(&self, on_change: F) -> notify::Result<()>
    where
        F: Fn(ConfigChanged) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if is_db_event(&event) {
                    let _ = tx.send(event);
                }
            }
        })?;

        watcher.watch(&self.config_dir, RecursiveMode::NonRecursive)?;

        let debounce = self.debounce_duration;
        let config_dir = self.config_dir.clone();

        // Debounce loop
        let mut last_event: Option<Instant> = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(_) => {
                    last_event = Some(Instant::now());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(last) = last_event {
                        if last.elapsed() >= debounce {
                            last_event = None;
                            on_change(ConfigChanged {
                                config_dir: config_dir.clone(),
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        Ok(())
    }
}

/// Check if a notify event relates to database.db* files.
fn is_db_event(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_)
    ) && event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("database.db"))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_event(path: &str, kind: EventKind) -> Event {
        Event {
            kind,
            paths: vec![PathBuf::from(path)],
            attrs: Default::default(),
        }
    }

    #[test]
    fn test_is_db_event_matches_database_files() {
        let event = mock_event("/some/path/database.db", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(is_db_event(&event));

        let event = mock_event("/some/path/database.db-wal", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(is_db_event(&event));

        let event = mock_event("/some/path/database.db-shm", EventKind::Create(notify::event::CreateKind::File));
        assert!(is_db_event(&event));
    }

    #[test]
    fn test_is_db_event_ignores_unrelated_files() {
        let event = mock_event("/some/path/config.json", EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)));
        assert!(!is_db_event(&event));
    }

    #[test]
    fn test_is_db_event_ignores_delete_events() {
        let event = mock_event("/some/path/database.db", EventKind::Remove(notify::event::RemoveKind::File));
        assert!(!is_db_event(&event));
    }
}
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test watcher`
Expected: All 3 tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement file watcher with debouncing for config changes"
```

---

### Task 6: Implement Folder Sync Provider

**Files:**
- Modify: `src-tauri/src/providers/folder.rs`

**Step 1: Write implementation with tests**

```rust
//! Cloud folder sync provider — copies config to/from a shared folder
//! (Dropbox, OneDrive, Google Drive, iCloud, or any synced directory).

use super::{ConfigSnapshot, ProviderError, ProviderResult, SyncMeta, SyncProvider};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

const META_FILE: &str = "sync_meta.json";

pub struct FolderProvider {
    sync_dir: PathBuf,
    device_name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredMeta {
    last_modified: chrono::DateTime<chrono::Utc>,
    device_name: String,
}

impl FolderProvider {
    pub fn new(sync_dir: PathBuf, device_name: String) -> Self {
        Self { sync_dir, device_name }
    }
}

#[async_trait::async_trait]
impl SyncProvider for FolderProvider {
    async fn push(&self, snapshot: &ConfigSnapshot) -> ProviderResult<()> {
        fs::create_dir_all(&self.sync_dir)?;
        fs::write(self.sync_dir.join("database.db"), &snapshot.db)?;
        if let Some(shm) = &snapshot.db_shm {
            fs::write(self.sync_dir.join("database.db-shm"), shm)?;
        }
        if let Some(wal) = &snapshot.db_wal {
            fs::write(self.sync_dir.join("database.db-wal"), wal)?;
        }
        let meta = StoredMeta {
            last_modified: Utc::now(),
            device_name: self.device_name.clone(),
        };
        let meta_json = serde_json::to_string_pretty(&meta)
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        fs::write(self.sync_dir.join(META_FILE), meta_json)?;
        Ok(())
    }

    async fn pull(&self) -> ProviderResult<ConfigSnapshot> {
        let db = fs::read(self.sync_dir.join("database.db"))
            .map_err(|_| ProviderError::NotFound)?;
        let db_shm = fs::read(self.sync_dir.join("database.db-shm")).ok();
        let db_wal = fs::read(self.sync_dir.join("database.db-wal")).ok();
        let meta = self.remote_meta().await?;
        Ok(ConfigSnapshot { db, db_shm, db_wal, meta })
    }

    async fn remote_meta(&self) -> ProviderResult<SyncMeta> {
        let meta_path = self.sync_dir.join(META_FILE);
        let meta_json = fs::read_to_string(&meta_path)
            .map_err(|_| ProviderError::NotFound)?;
        let stored: StoredMeta = serde_json::from_str(&meta_json)
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        Ok(SyncMeta {
            last_modified: stored.last_modified,
            device_name: stored.device_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_push_creates_files() {
        let tmp = TempDir::new().unwrap();
        let provider = FolderProvider::new(tmp.path().to_path_buf(), "test-pc".into());
        let snapshot = ConfigSnapshot {
            db: b"db-content".to_vec(),
            db_shm: Some(b"shm-content".to_vec()),
            db_wal: Some(b"wal-content".to_vec()),
            meta: SyncMeta {
                last_modified: Utc::now(),
                device_name: "test-pc".into(),
            },
        };

        provider.push(&snapshot).await.unwrap();

        assert_eq!(fs::read(tmp.path().join("database.db")).unwrap(), b"db-content");
        assert_eq!(fs::read(tmp.path().join("database.db-shm")).unwrap(), b"shm-content");
        assert_eq!(fs::read(tmp.path().join("database.db-wal")).unwrap(), b"wal-content");
        assert!(tmp.path().join(META_FILE).exists());
    }

    #[tokio::test]
    async fn test_push_then_pull_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let provider = FolderProvider::new(tmp.path().to_path_buf(), "my-pc".into());
        let snapshot = ConfigSnapshot {
            db: b"roundtrip-db".to_vec(),
            db_shm: None,
            db_wal: Some(b"roundtrip-wal".to_vec()),
            meta: SyncMeta {
                last_modified: Utc::now(),
                device_name: "my-pc".into(),
            },
        };

        provider.push(&snapshot).await.unwrap();
        let pulled = provider.pull().await.unwrap();

        assert_eq!(pulled.db, b"roundtrip-db");
        assert!(pulled.db_shm.is_none());
        assert_eq!(pulled.db_wal.unwrap(), b"roundtrip-wal");
        assert_eq!(pulled.meta.device_name, "my-pc");
    }

    #[tokio::test]
    async fn test_pull_not_found() {
        let tmp = TempDir::new().unwrap();
        let provider = FolderProvider::new(tmp.path().join("empty").to_path_buf(), "pc".into());
        let result = provider.pull().await;
        assert!(matches!(result, Err(ProviderError::NotFound)));
    }

    #[tokio::test]
    async fn test_remote_meta() {
        let tmp = TempDir::new().unwrap();
        let provider = FolderProvider::new(tmp.path().to_path_buf(), "gaming-rig".into());
        let snapshot = ConfigSnapshot {
            db: b"data".to_vec(),
            db_shm: None,
            db_wal: None,
            meta: SyncMeta {
                last_modified: Utc::now(),
                device_name: "gaming-rig".into(),
            },
        };

        provider.push(&snapshot).await.unwrap();
        let meta = provider.remote_meta().await.unwrap();
        assert_eq!(meta.device_name, "gaming-rig");
    }
}
```

Also add `tokio` to dev-dependencies for `#[tokio::test]`:
```toml
[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test folder`
Expected: All 4 tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement folder sync provider with push/pull/meta"
```

---

### Task 7: Implement Hosted API Sync Provider

**Files:**
- Modify: `src-tauri/src/providers/hosted.rs`

**Step 1: Write implementation**

```rust
//! Hosted API sync provider — communicates with the Mac Mini sync server.

use super::{ConfigSnapshot, ProviderError, ProviderResult, SyncMeta, SyncProvider};
use reqwest::Client;

pub struct HostedProvider {
    client: Client,
    api_url: String,
    api_key: String,
    device_name: String,
}

#[derive(serde::Deserialize)]
struct MetaResponse {
    last_modified: chrono::DateTime<chrono::Utc>,
    device_name: String,
}

#[derive(serde::Deserialize)]
struct PullResponse {
    db: Vec<u8>,
    db_shm: Option<Vec<u8>>,
    db_wal: Option<Vec<u8>>,
    last_modified: chrono::DateTime<chrono::Utc>,
    device_name: String,
}

impl HostedProvider {
    pub fn new(api_url: String, api_key: String, device_name: String) -> Self {
        Self {
            client: Client::new(),
            api_url,
            api_key,
            device_name,
        }
    }
}

#[async_trait::async_trait]
impl SyncProvider for HostedProvider {
    async fn push(&self, snapshot: &ConfigSnapshot) -> ProviderResult<()> {
        let form = reqwest::multipart::Form::new()
            .part("db", reqwest::multipart::Part::bytes(snapshot.db.clone()).file_name("database.db"))
            .part("db_shm", reqwest::multipart::Part::bytes(
                snapshot.db_shm.clone().unwrap_or_default()
            ).file_name("database.db-shm"))
            .part("db_wal", reqwest::multipart::Part::bytes(
                snapshot.db_wal.clone().unwrap_or_default()
            ).file_name("database.db-wal"))
            .text("device_name", self.device_name.clone());

        let resp = self.client
            .put(&format!("{}/sync", self.api_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!("HTTP {}", resp.status())));
        }
        Ok(())
    }

    async fn pull(&self) -> ProviderResult<ConfigSnapshot> {
        let resp = self.client
            .get(&format!("{}/sync", self.api_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!("HTTP {}", resp.status())));
        }

        let body: PullResponse = resp.json().await?;
        Ok(ConfigSnapshot {
            db: body.db,
            db_shm: if body.db_shm.as_ref().map(|v| v.is_empty()).unwrap_or(true) { None } else { body.db_shm },
            db_wal: if body.db_wal.as_ref().map(|v| v.is_empty()).unwrap_or(true) { None } else { body.db_wal },
            meta: SyncMeta {
                last_modified: body.last_modified,
                device_name: body.device_name,
            },
        })
    }

    async fn remote_meta(&self) -> ProviderResult<SyncMeta> {
        let resp = self.client
            .get(&format!("{}/sync/meta", self.api_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ProviderError::NotFound);
        }
        if !resp.status().is_success() {
            return Err(ProviderError::Other(format!("HTTP {}", resp.status())));
        }

        let meta: MetaResponse = resp.json().await?;
        Ok(SyncMeta {
            last_modified: meta.last_modified,
            device_name: meta.device_name,
        })
    }
}
```

Note: No unit tests for hosted provider — it requires a running server. It will be tested via integration tests in Phase 5 when the server exists.

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement hosted API sync provider"
```

---

### Task 8: Implement Sync Engine (orchestrator)

**Files:**
- Modify: `src-tauri/src/sync_engine.rs`

**Step 1: Write implementation with tests**

```rust
//! Core sync orchestration — coordinates watcher, safety, backup, and provider.

use crate::backup::BackupManager;
use crate::config::AppConfig;
use crate::providers::{ConfigSnapshot, ProviderError, SyncMeta, SyncProvider};
use crate::safety::{validate_sqlite_header, SafetyCheck, SafetyGuard};
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SyncEngine {
    config: AppConfig,
    provider: Arc<dyn SyncProvider>,
    backup_manager: BackupManager,
    safety: Mutex<SafetyGuard>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncResult {
    Pushed,
    Pulled { from_device: String },
    ConflictResolved { winner: String, backup_label: String },
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
        match safety.is_safe_to_write(&self.config.steelseries_db_path) {
            SafetyCheck::Safe => {}
            SafetyCheck::GGRunning => return Ok(SyncResult::Skipped(SkipReason::GGRunning)),
            SafetyCheck::FileLocked => return Ok(SyncResult::Skipped(SkipReason::FileLocked)),
            SafetyCheck::NoConfig => {} // OK to write even if no existing config
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

        self.write_local_config(&remote)?;
        Ok(SyncResult::Pulled {
            from_device: remote.meta.device_name,
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
                    // Local is newer — push
                    self.backup_manager
                        .create_backup(&self.config.steelseries_db_path, "pre-push")?;
                    self.push_to_remote().await
                } else if remote.last_modified > local_ts {
                    // Remote is newer — pull
                    self.pull_from_remote().await
                } else {
                    Ok(SyncResult::Skipped(SkipReason::AlreadyInSync))
                }
            }
            // Only local exists — push
            (true, None) => self.push_to_remote().await,
            // Only remote exists — pull
            (false, Some(_)) => self.pull_from_remote().await,
            // Neither exists
            (false, None) => Ok(SyncResult::Skipped(SkipReason::NoLocalConfig)),
        }
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
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement sync engine with push/pull/conflict resolution"
```

---

## Phase 3: Tauri Commands & Frontend

### Task 9: Wire Tauri Commands

**Files:**
- Modify: `src-tauri/src/lib.rs` (or `main.rs`)

**Step 1: Create Tauri commands**

Expose sync engine operations to the frontend via `#[tauri::command]`:

```rust
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

struct AppState {
    engine: Arc<SyncEngine>,
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
    state.engine.backups()
        .list_backups()
        .map(|entries| entries.iter().map(|e| e.name.clone()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<String, String> {
    // Return current config as JSON
    Ok("{}".to_string()) // placeholder — will return actual config
}
```

Register in the Tauri builder:
```rust
.invoke_handler(tauri::generate_handler![
    sync_now, push_now, pull_now, list_backups, get_config
])
```

Initialize `AppState` with a `SyncEngine` using default config and the folder provider.

**Step 2: Verify it compiles and launches**

Run: `npm run tauri dev`
Expected: App launches without errors.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: wire Tauri commands for sync, push, pull, backups"
```

---

### Task 10: Build React Frontend — Status Page

**Files:**
- Modify: `src/App.tsx`
- Create: `src/pages/Status.tsx`

**Step 1: Build status page**

Simple page showing:
- Current sync status (idle/syncing/error/offline)
- Last sync timestamp and device name
- "Sync Now" button
- "Push" / "Pull" buttons

Use `@tauri-apps/api/core` to invoke commands.

**Step 2: Verify it renders**

Run: `npm run tauri dev`
Expected: Status page shows with working buttons.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add status page with sync controls"
```

---

### Task 11: Build React Frontend — Settings Page

**Files:**
- Create: `src/pages/Settings.tsx`

**Step 1: Build settings page**

Form with:
- SteelSeries config path (text input, prefilled with default)
- Provider selector (Folder / Hosted)
- Folder provider: sync directory path
- Hosted provider: API URL + API key
- Device name
- Max backups slider
- Debounce seconds

Save config via Tauri command.

**Step 2: Verify it renders and saves**

Run: `npm run tauri dev`
Expected: Settings form renders, values persist.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add settings page with provider configuration"
```

---

### Task 12: Build React Frontend — Backup Browser

**Files:**
- Create: `src/pages/BackupBrowser.tsx`

**Step 1: Build backup browser**

- List of backups (name, date) from `list_backups` command
- "Restore" button per entry (calls a `restore_backup` Tauri command)
- Confirmation dialog before restore

**Step 2: Verify it renders**

Run: `npm run tauri dev`
Expected: Backup list shows, restore works.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add backup browser with restore functionality"
```

---

### Task 13: System Tray Integration

**Files:**
- Modify: `src-tauri/src/lib.rs` (or `main.rs`)

**Step 1: Add system tray**

Use Tauri's `SystemTray` API:
- Tray icon (green = synced, yellow = syncing, red = error, gray = offline)
- Right-click menu: Sync Now, Open Settings, View Backups, Quit
- Tooltip: last sync status

**Step 2: Verify tray appears**

Run: `npm run tauri dev`
Expected: Tray icon appears with working menu.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add system tray with sync status and quick actions"
```

---

## Phase 4: Background Watcher

### Task 14: Start File Watcher on App Launch

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Spawn watcher thread on startup**

On Tauri `setup`, spawn a thread running `ConfigWatcher::watch()`. When a debounced change is detected, trigger `sync_engine.push_to_remote()`. Emit Tauri events to update the frontend status.

**Step 2: Test end-to-end**

1. Launch app
2. Modify a file in a test config directory
3. Verify sync is triggered and tray updates

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: start file watcher on app launch, auto-push on changes"
```

---

### Task 15: Add Inbound Sync Polling

**Files:**
- Modify: `src-tauri/src/sync_engine.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Add polling loop**

Spawn a tokio task that periodically (every 30s) checks `provider.remote_meta()` and compares with local. If remote is newer, trigger pull (respecting safety guard).

**Step 2: Test end-to-end**

1. Launch app on two machines (or simulate with two config dirs)
2. Push from machine A
3. Verify machine B pulls within 30s

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: add inbound sync polling every 30s"
```

---

## Phase 5: Hosted Sync Server

### Task 16: Scaffold NestJS Server

**Files:**
- Create: `server/` directory with NestJS project

**Step 1: Initialize NestJS**

```bash
cd "/Users/marlinjai/software dev/steelseries-sync"
npx @nestjs/cli new server --package-manager npm --skip-git
```

**Step 2: Commit**

```bash
git add -A
git commit -m "feat: scaffold NestJS server for hosted sync tier"
```

---

### Task 17: Implement Sync Endpoints

**Files:**
- Create: `server/src/sync/sync.module.ts`
- Create: `server/src/sync/sync.controller.ts`
- Create: `server/src/sync/sync.service.ts`

**Step 1: Write failing tests**

```typescript
// server/src/sync/sync.controller.spec.ts
describe('SyncController', () => {
  it('PUT /sync — uploads config files', async () => { ... });
  it('GET /sync — downloads config files', async () => { ... });
  it('GET /sync/meta — returns metadata', async () => { ... });
  it('GET /sync — returns 404 when no config exists', async () => { ... });
});
```

**Step 2: Implement endpoints**

- `PUT /sync` — accepts multipart upload (db, db_shm, db_wal, device_name), stores to `/data/users/{userId}/`
- `GET /sync` — returns JSON with base64-encoded files + metadata
- `GET /sync/meta` — returns `{ last_modified, device_name }`

User ID comes from JWT in Authorization header.

**Step 3: Run tests**

Run: `cd server && npm test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: implement sync endpoints (PUT/GET /sync, GET /sync/meta)"
```

---

### Task 18: Implement Authentication

**Files:**
- Create: `server/src/auth/auth.module.ts`
- Create: `server/src/auth/auth.controller.ts`
- Create: `server/src/auth/auth.service.ts`

**Step 1: Implement simple email + password auth**

- `POST /auth/register` — create account, return JWT
- `POST /auth/login` — authenticate, return JWT
- JWT guard on `/sync/*` endpoints
- Store users in a simple JSON file or SQLite (this is a small-scale service)

**Step 2: Test**

Run: `cd server && npm test`
Expected: Auth tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: implement email/password auth with JWT"
```

---

### Task 19: Mac Mini Deployment Script

**Files:**
- Create: `scripts/setup-mac-mini.sh`

**Step 1: Write setup script (modeled on Lola Stories)**

```bash
#!/bin/bash
# SteelSeries Sync — Mac Mini Setup
# Installs and configures the hosted sync server with PM2 + Cloudflare Tunnel.
# Modeled after the Lola Stories setup at:
#   /Users/marlinjai/software dev/lola-stories/scripts/setup-mac-mini.sh

# 1. Install dependencies (Node, PM2, cloudflared)
# 2. Build the NestJS server
# 3. Create Cloudflare Tunnel (steelseries-sync)
# 4. Configure tunnel ingress (sync.yourdomain.com -> localhost:3001)
# 5. Start server + tunnel via PM2
# 6. Set up auto-deploy cron job
# 7. pm2 save && pm2 startup
```

**Step 2: Commit**

```bash
git add -A
git commit -m "feat: add Mac Mini deployment script (PM2 + Cloudflare Tunnel)"
```

---

## Phase 6: Polish & Release

### Task 20: Add README and LICENSE

**Files:**
- Create: `README.md`
- Create: `LICENSE` (MIT)

Include: description, features, installation, configuration, building from source, hosted tier, contributing.

**Step 1: Commit**

```bash
git add -A
git commit -m "docs: add README and MIT license"
```

---

### Task 21: Build and Test Release Binary

**Step 1: Build release**

```bash
npm run tauri build
```

**Step 2: Test the built app**

- Install on Windows (or Mac for now)
- Configure a test sync folder
- Verify push/pull/backup/restore all work
- Verify tray icon and notifications

**Step 3: Tag release**

```bash
git tag v0.1.0
git push origin main --tags
```
