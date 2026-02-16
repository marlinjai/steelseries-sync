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
