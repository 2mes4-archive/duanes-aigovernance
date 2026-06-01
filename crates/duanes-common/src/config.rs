use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuanesConfig {
    pub engine: EngineConfig,
    pub storage: StorageConfig,
    pub governance: GovernanceConfig,
    pub collector: Option<CollectorConfig>,
    pub local_runner: Option<LocalRunnerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub default_provider: String,
    pub request_timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            request_timeout_secs: 120,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub path: String,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    Sqlite,
    InMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub store_prompt: bool,
    pub store_response: bool,
    pub store_cost: bool,
    pub store_provider: bool,
    pub store_model: bool,
    pub store_latency: bool,
    pub store_identity: bool,
    pub mask_pii: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            store_prompt: true,
            store_response: true,
            store_cost: true,
            store_provider: true,
            store_model: true,
            store_latency: true,
            store_identity: true,
            mask_pii: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub enabled: bool,
    pub max_tokens_per_request: Option<u64>,
    pub max_tokens_per_user_daily: Option<u64>,
    pub max_cost_per_user_daily: Option<f64>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked_patterns: Option<Vec<String>>,
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_tokens_per_request: None,
            max_tokens_per_user_daily: None,
            max_cost_per_user_daily: None,
            allowed_models: None,
            blocked_patterns: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub sync_interval_secs: u64,
    pub batch_size: usize,
    pub retry_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRunnerConfig {
    pub enabled: bool,
    pub models_path: String,
    pub default_backend: LocalInferenceBackend,
    pub max_concurrent_inferences: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalInferenceBackend {
    CoreMl,
    Onnx,
    Nnapi,
    Auto,
}

impl DuanesConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
