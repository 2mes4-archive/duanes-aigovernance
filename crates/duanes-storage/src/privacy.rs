use duanes_common::config::PrivacyConfig;
use duanes_common::models::InferenceRecord;

pub struct PrivacyFilter {
    config: PrivacyConfig,
}

impl PrivacyFilter {
    pub fn new(config: PrivacyConfig) -> Self {
        Self { config }
    }

    pub fn apply(&self, record: &InferenceRecord) -> FilteredInferenceRecord {
        FilteredInferenceRecord {
            id: record.id,
            request_id: record.request_id,
            timestamp: record.timestamp,
            identity: if self.config.store_identity {
                record.identity.clone()
            } else {
                None
            },
            provider: if self.config.store_provider {
                record.provider.clone()
            } else {
                "[REDACTED]".to_string()
            },
            model: if self.config.store_model {
                record.model.clone()
            } else {
                "[REDACTED]".to_string()
            },
            execution_mode: record.execution_mode.clone(),
            prompt_hash: record.prompt_hash.clone(),
            prompt_text: if self.config.store_prompt {
                if self.config.mask_pii {
                    Some(Self::mask_pii(&record.prompt_text.clone().unwrap_or_default()))
                } else {
                    record.prompt_text.clone()
                }
            } else {
                None
            },
            response_text: if self.config.store_response {
                record.response_text.clone()
            } else {
                None
            },
            usage: record.usage.clone(),
            latency: if self.config.store_latency {
                record.latency.clone()
            } else {
                duanes_common::models::LatencyMetrics {
                    time_to_first_token_ms: None,
                    total_latency_ms: 0,
                    routing_overhead_ms: 0,
                }
            },
            cost: if self.config.store_cost {
                record.cost.clone()
            } else {
                duanes_common::models::CostMetrics::default()
            },
            governance_result: record.governance_result.clone(),
            metadata: record.metadata.clone(),
        }
    }

    fn mask_pii(text: &str) -> String {
        let mut result = text.to_string();
        let patterns = [
            (r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b", "[PHONE]"),
            (r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "[EMAIL]"),
            (r"\b\d{3}[-.]?\d{2}[-.]?\d{4}\b", "[SSN]"),
            (r"\b\d{16}\b", "[CC]"),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }

        result
    }
}

pub struct FilteredInferenceRecord {
    pub id: uuid::Uuid,
    pub request_id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub identity: Option<duanes_common::models::CallerIdentity>,
    pub provider: String,
    pub model: String,
    pub execution_mode: duanes_common::models::ExecutionMode,
    pub prompt_hash: Option<String>,
    pub prompt_text: Option<String>,
    pub response_text: Option<String>,
    pub usage: duanes_common::models::TokenUsage,
    pub latency: duanes_common::models::LatencyMetrics,
    pub cost: duanes_common::models::CostMetrics,
    pub governance_result: duanes_common::models::GovernanceResult,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}
