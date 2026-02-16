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
