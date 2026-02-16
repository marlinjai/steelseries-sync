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
