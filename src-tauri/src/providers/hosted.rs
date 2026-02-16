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
