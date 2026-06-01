use async_trait::async_trait;
use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::{InferenceRequest, InferenceResponse};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait InferenceBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse>;
    fn supports_model(&self, model: &str) -> bool;
    fn list_models(&self) -> Vec<String>;
}

pub struct CoreMlBackend {
    models_path: String,
    loaded_models: Arc<RwLock<HashMap<String, ()>>>,
}

impl CoreMlBackend {
    pub fn new(models_path: String) -> Self {
        Self {
            models_path,
            loaded_models: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl InferenceBackend for CoreMlBackend {
    fn name(&self) -> &str {
        "coreml"
    }

    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        Err(DuanesError::LocalInference(
            "CoreML backend is a stub. Enable the 'coreml' feature and link the native library.".to_string()
        ))
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("local/") || model.contains("coreml")
    }

    fn list_models(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct OnnxBackend {
    models_path: String,
    loaded_models: Arc<RwLock<HashMap<String, ()>>>,
}

impl OnnxBackend {
    pub fn new(models_path: String) -> Self {
        Self {
            models_path,
            loaded_models: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl InferenceBackend for OnnxBackend {
    fn name(&self) -> &str {
        "onnx"
    }

    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        Err(DuanesError::LocalInference(
            "ONNX backend is a stub. Enable the 'onnx' feature and link the ONNX Runtime.".to_string()
        ))
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("local/") || model.contains("onnx")
    }

    fn list_models(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct NnapiBackend {
    models_path: String,
}

impl NnapiBackend {
    pub fn new(models_path: String) -> Self {
        Self { models_path }
    }
}

#[async_trait]
impl InferenceBackend for NnapiBackend {
    fn name(&self) -> &str {
        "nnapi"
    }

    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        Err(DuanesError::LocalInference(
            "NNAPI backend is a stub. Enable the 'nnapi' feature and link the Android NDK.".to_string()
        ))
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("local/") || model.contains("nnapi")
    }

    fn list_models(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct GenericHttpBackend {
    base_url: String,
    client: reqwest::Client,
}

impl GenericHttpBackend {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client for generic backend");

        Self { base_url, client }
    }
}

#[async_trait]
impl InferenceBackend for GenericHttpBackend {
    fn name(&self) -> &str {
        "generic_http"
    }

    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DuanesError::LocalInference(format!("Local model server error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(DuanesError::LocalInference(
                format!("Local model server returned {}: {}", status, body)
            ));
        }

        response.json().await
            .map_err(|e| DuanesError::LocalInference(format!("Failed to parse response: {}", e)))
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("local/")
    }

    fn list_models(&self) -> Vec<String> {
        Vec::new()
    }
}
