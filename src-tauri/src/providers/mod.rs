//! Provider adapter interface and implementations.

pub mod folder;
pub mod hosted;

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
