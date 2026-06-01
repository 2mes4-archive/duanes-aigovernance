use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::{InferenceRequest, InferenceResponse};
use crate::backends::InferenceBackend;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LocalModelRunner {
    backends: Arc<RwLock<Vec<Box<dyn InferenceBackend>>>>,
    max_concurrent: usize,
}

impl LocalModelRunner {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            backends: Arc::new(RwLock::new(Vec::new())),
            max_concurrent,
        }
    }

    pub async fn register_backend(&self, backend: Box<dyn InferenceBackend>) {
        let mut backends = self.backends.write().await;
        tracing::info!(backend = %backend.name(), "Registered local inference backend");
        backends.push(backend);
    }

    pub async fn list_available_models(&self) -> Vec<(String, String)> {
        let backends = self.backends.read().await;
        let mut models = Vec::new();
        for backend in backends.iter() {
            for model in backend.list_models() {
                models.push((backend.name().to_string(), model));
            }
        }
        models
    }

    pub async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        let backends = self.backends.read().await;

        for backend in backends.iter() {
            if backend.supports_model(&request.model) {
                let name = backend.name().to_string();
                let start = std::time::Instant::now();

                let response = backend.inference(request).await?;

                let elapsed = start.elapsed();
                tracing::info!(
                    backend = %name,
                    elapsed_ms = elapsed.as_millis(),
                    "Local inference completed"
                );

                return Ok(response);
            }
        }

        Err(DuanesError::LocalInference(format!(
            "No local backend supports model '{}'", request.model
        )))
    }

    pub fn supports_model(&self, _model: &str) -> bool {
        false
    }

    pub fn name(&self) -> &str {
        "local_runner"
    }
}

impl Clone for LocalModelRunner {
    fn clone(&self) -> Self {
        Self {
            backends: self.backends.clone(),
            max_concurrent: self.max_concurrent,
        }
    }
}
