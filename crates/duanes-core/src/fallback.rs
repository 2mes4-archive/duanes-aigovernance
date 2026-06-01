use duanes_common::errors::DuanesResult;
use duanes_common::models::InferenceResponse;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct FallbackConfig {
    pub enabled: bool,
    pub prefer_local_on_failure: bool,
    pub max_fallback_attempts: u32,
    pub fallback_models: Vec<String>,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_local_on_failure: true,
            max_fallback_attempts: 2,
            fallback_models: Vec::new(),
        }
    }
}

pub struct FallbackManager {
    config: Arc<RwLock<FallbackConfig>>,
    failure_counter: Arc<RwLock<dashmap::DashMap<String, u32>>>,
}

impl FallbackManager {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(FallbackConfig::default())),
            failure_counter: Arc::new(RwLock::new(dashmap::DashMap::new())),
        }
    }

    pub fn with_config(config: FallbackConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            failure_counter: Arc::new(RwLock::new(dashmap::DashMap::new())),
        }
    }

    pub async fn should_fallback(&self, provider: &str, model: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return false;
        }

        let key = format!("{}:{}", provider, model);
        let counter = self.failure_counter.read().await;
        let failures = counter.get(&key).map(|v| *v).unwrap_or(0);

        failures >= 3
    }

    pub async fn record_failure(&self, provider: &str, model: &str) {
        let key = format!("{}:{}", provider, model);
        let counter = self.failure_counter.read().await;
        counter.entry(key).and_modify(|v| *v += 1).or_insert(1);
    }

    pub async fn record_success(&self, provider: &str, model: &str) {
        let key = format!("{}:{}", provider, model);
        let counter = self.failure_counter.read().await;
        counter.remove(&key);
    }

    pub async fn get_fallback_model(&self, original_model: &str) -> Option<String> {
        let config = self.config.read().await;
        config.fallback_models.first().cloned()
    }

    pub async fn update_config(&self, new_config: FallbackConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }
}
