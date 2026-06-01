use duanes_common::config::CollectorConfig;
use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::{InferenceRecord, SyncPayload};
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};
use uuid::Uuid;

pub struct CollectorSync {
    config: Arc<RwLock<CollectorConfig>>,
    client: Client,
    source_id: String,
    running: Arc<RwLock<bool>>,
}

impl CollectorSync {
    pub fn new(config: CollectorConfig, source_id: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build sync client");

        Self {
            config: Arc::new(RwLock::new(config)),
            client,
            source_id,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self, storage: Arc<crate::sqlite::SqliteStorage>) {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        let config = self.config.read().await;
        let mut ticker = interval(Duration::from_secs(config.sync_interval_secs));
        drop(config);

        loop {
            let is_running = *self.running.read().await;
            if !is_running {
                break;
            }

            ticker.tick().await;

            if let Err(e) = self.sync_batch(&storage).await {
                tracing::error!(error = %e, "Failed to sync batch with collector");
            }
        }
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    async fn sync_batch(&self, storage: &crate::sqlite::SqliteStorage) -> DuanesResult<()> {
        let config = self.config.read().await;
        let records = storage.get_pending_sync(config.batch_size).await?;

        if records.is_empty() {
            return Ok(());
        }

        let record_ids: Vec<Uuid> = records.iter().map(|r| r.id).collect();

        let payload = SyncPayload {
            source_id: self.source_id.clone(),
            records,
            batch_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
        };

        let mut request = self.client
            .post(format!("{}/api/v1/ingest", config.endpoint.trim_end_matches('/')));

        if let Some(ref api_key) = config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| DuanesError::CollectorSync(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DuanesError::CollectorSync(
                format!("Collector returned {}: {}", status, body)
            ));
        }

        storage.mark_synced(&record_ids).await?;
        tracing::info!(
            batch_id = %payload.batch_id,
            records = record_ids.len(),
            "Successfully synced batch with collector"
        );

        Ok(())
    }

    pub async fn update_config(&self, new_config: CollectorConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }
}
