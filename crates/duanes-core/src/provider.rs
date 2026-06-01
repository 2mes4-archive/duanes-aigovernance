use async_trait::async_trait;
use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::{InferenceRequest, InferenceResponse, ProviderConfig, StreamChunk};
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use reqwest::Client;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse>;
    async fn inference_stream(
        &self,
        request: InferenceRequest,
    ) -> DuanesResult<Pin<Box<dyn Stream<Item = DuanesResult<StreamChunk>> + Send>>>;
    fn supports_model(&self, model: &str) -> bool;
    fn clone_box(&self) -> Box<dyn Provider>;
}

impl Clone for Box<dyn Provider> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct HttpProvider {
    config: ProviderConfig,
    client: Client,
}

impl HttpProvider {
    pub fn new(config: ProviderConfig, client: Client) -> Self {
        Self { config, client }
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }
}

#[async_trait]
impl Provider for HttpProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn inference(&self, mut request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        let url = self.build_url("/v1/chat/completions");
        request.stream = None;

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DuanesError::ProviderUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DuanesError::ProviderUnavailable(
                format!("Provider returned {}: {}", status, body)
            ));
        }

        let inference_response: InferenceResponse = response
            .json()
            .await
            .map_err(|e| DuanesError::Serialization(e.to_string()))?;

        Ok(inference_response)
    }

    async fn inference_stream(
        &self,
        mut request: InferenceRequest,
    ) -> DuanesResult<Pin<Box<dyn Stream<Item = DuanesResult<StreamChunk>> + Send>>> {
        let url = self.build_url("/v1/chat/completions");
        request.stream = Some(true);

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DuanesError::ProviderUnavailable(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DuanesError::ProviderUnavailable(
                format!("Provider returned {}: {}", status, body)
            ));
        }

        let stream = response.bytes_stream()
            .eventsource()
            .map(|result| {
                match result {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            return Err(DuanesError::Internal("Stream complete".to_string()));
                        }
                        let chunk: StreamChunk = serde_json::from_str(&event.data)
                            .map_err(|e| DuanesError::Serialization(e.to_string()))?;
                        Ok(chunk)
                    }
                    Err(e) => Err(DuanesError::ProviderUnavailable(e.to_string())),
                }
            });

        Ok(Box::pin(stream))
    }

    fn supports_model(&self, model: &str) -> bool {
        self.config.models.iter().any(|m| m.model_id == model)
    }

    fn clone_box(&self) -> Box<dyn Provider> {
        Box::new(HttpProvider::new(self.config.clone(), self.client.clone()))
    }
}

// --- Provider Registry ---

pub struct ProviderRegistry {
    providers: Arc<RwLock<DashMapProviderStore>>,
    client: Client,
}

struct DashMapProviderStore {
    providers: dashmap::DashMap<String, Box<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(DashMapProviderStore {
                providers: dashmap::DashMap::new(),
            })),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn register(&self, name: String, provider: Box<dyn Provider>) {
        let store = self.providers.read().await;
        store.providers.insert(name, provider);
    }

    pub async fn get(&self, name: &str) -> Option<ProviderRef> {
        let store = self.providers.read().await;
        store.providers.get(name).map(|p| ProviderRef {
            inner: p.value().clone(),
        })
    }

    pub async fn list_providers(&self) -> Vec<String> {
        let store = self.providers.read().await;
        store.providers.iter().map(|e| e.key().clone()).collect()
    }

    pub async fn register_from_config(&self, config: ProviderConfig) -> DuanesResult<()> {
        let provider = HttpProvider::new(config, self.client.clone());
        self.register(provider.name().to_string(), Box::new(provider)).await;
        Ok(())
    }
}

pub struct ProviderRef {
    inner: Box<dyn Provider>,
}

impl ProviderRef {
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn supports_model(&self, model: &str) -> bool {
        self.inner.supports_model(model)
    }

    pub async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        self.inner.inference(request).await
    }

    pub async fn inference_stream(
        &self,
        request: InferenceRequest,
    ) -> DuanesResult<Pin<Box<dyn Stream<Item = DuanesResult<StreamChunk>> + Send>>> {
        self.inner.inference_stream(request).await
    }
}
