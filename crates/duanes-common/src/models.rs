use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    #[serde(skip_deserializing, default = "Uuid::new_v4")]
    pub id: Uuid,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl InferenceRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            id: Uuid::new_v4(),
            model: model.into(),
            messages,
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: None,
            stop: None,
            user: None,
            extra: serde_json::Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: ContentPart,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentPart {
    Text(String),
    MultiPart(Vec<ContentBlock>),
}

impl ContentPart {
    pub fn text(&self) -> String {
        match self {
            ContentPart::Text(t) => t.clone(),
            ContentPart::MultiPart(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRecord {
    pub id: Uuid,
    pub request_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub identity: Option<CallerIdentity>,
    pub provider: String,
    pub model: String,
    pub execution_mode: ExecutionMode,
    pub prompt_hash: Option<String>,
    pub prompt_text: Option<String>,
    pub response_text: Option<String>,
    pub usage: TokenUsage,
    pub latency: LatencyMetrics,
    pub cost: CostMetrics,
    pub governance_result: GovernanceResult,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerIdentity {
    pub user_id: String,
    pub app_id: Option<String>,
    pub team_id: Option<String>,
    pub org_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    CloudProxy,
    LocalInference,
    Fallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub time_to_first_token_ms: Option<u64>,
    pub total_latency_ms: u64,
    pub routing_overhead_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostMetrics {
    pub prompt_cost: f64,
    pub completion_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

impl Default for CostMetrics {
    fn default() -> Self {
        Self {
            prompt_cost: 0.0,
            completion_cost: 0.0,
            total_cost: 0.0,
            currency: "USD".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceResult {
    pub allowed: bool,
    pub rules_evaluated: Vec<String>,
    pub violation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub models: Vec<ModelPricing>,
    pub rate_limit: Option<RateLimitConfig>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model_id: String,
    pub prompt_cost_per_million: f64,
    pub completion_cost_per_million: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub source_id: String,
    pub records: Vec<InferenceRecord>,
    pub batch_id: Uuid,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}
