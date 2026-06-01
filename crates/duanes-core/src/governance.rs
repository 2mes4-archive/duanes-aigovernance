use duanes_common::config::GovernanceConfig;
use duanes_common::errors::DuanesResult;
use duanes_common::models::{CallerIdentity, GovernanceResult, InferenceRequest};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct GovernanceEngine {
    config: Arc<RwLock<GovernanceConfig>>,
    daily_usage: Arc<RwLock<DailyUsageTracker>>,
}

#[derive(Debug, Default)]
struct DailyUsageTracker {
    tokens_by_user: dashmap::DashMap<String, u64>,
    cost_by_user: dashmap::DashMap<String, f64>,
}

impl GovernanceEngine {
    pub fn new(config: GovernanceConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            daily_usage: Arc::new(RwLock::new(DailyUsageTracker::default())),
        }
    }

    pub async fn evaluate(
        &self,
        request: &InferenceRequest,
        identity: Option<&CallerIdentity>,
    ) -> DuanesResult<GovernanceResult> {
        let config = self.config.read().await;
        let mut rules_evaluated = Vec::new();
        let mut estimated_tokens = 0u64;

        if !config.enabled {
            return Ok(GovernanceResult {
                allowed: true,
                rules_evaluated: vec!["governance_disabled".to_string()],
                violation: None,
            });
        }

        if let Some(max) = config.max_tokens_per_request {
            estimated_tokens = crate::openai::OpenAICompatLayer::count_prompt_tokens(request);
            if let Some(max_tokens) = request.max_tokens {
                estimated_tokens += max_tokens;
            }
            rules_evaluated.push("max_tokens_per_request".to_string());
            if estimated_tokens > max {
                return Ok(GovernanceResult {
                    allowed: false,
                    rules_evaluated,
                    violation: Some(format!(
                        "Request exceeds per-request token limit: {} > {}",
                        estimated_tokens, max
                    )),
                });
            }
        }

        if let Some(allowed) = &config.allowed_models {
            rules_evaluated.push("allowed_models".to_string());
            if !allowed.iter().any(|m| request.model.contains(m)) {
                return Ok(GovernanceResult {
                    allowed: false,
                    rules_evaluated,
                    violation: Some(format!("Model '{}' is not in the allowed list", request.model)),
                });
            }
        }

        if let Some(blocked) = &config.blocked_patterns {
            rules_evaluated.push("blocked_patterns".to_string());
            for msg in &request.messages {
                let text = msg.content.text().to_lowercase();
                for pattern in blocked {
                    if text.contains(&pattern.to_lowercase()) {
                        return Ok(GovernanceResult {
                            allowed: false,
                            rules_evaluated,
                            violation: Some(format!("Prompt contains blocked pattern: '{}'", pattern)),
                        });
                    }
                }
            }
        }

        if let (Some(identity), Some(max_daily_tokens)) = (identity, config.max_tokens_per_user_daily) {
            rules_evaluated.push("daily_token_limit".to_string());
            let usage = self.daily_usage.read().await;
            let current = usage.tokens_by_user.get(&identity.user_id).map(|v| *v).unwrap_or(0);
            if current + estimated_tokens > max_daily_tokens {
                return Ok(GovernanceResult {
                    allowed: false,
                    rules_evaluated,
                    violation: Some(format!(
                        "User '{}' exceeds daily token budget: {} used of {}",
                        identity.user_id, current, max_daily_tokens
                    )),
                });
            }
        }

        if let (Some(identity), Some(max_daily_cost)) = (identity, config.max_cost_per_user_daily) {
            rules_evaluated.push("daily_cost_limit".to_string());
            let usage = self.daily_usage.read().await;
            let current = usage.cost_by_user.get(&identity.user_id).map(|v| *v).unwrap_or(0.0);
            if current >= max_daily_cost {
                return Ok(GovernanceResult {
                    allowed: false,
                    rules_evaluated,
                    violation: Some(format!(
                        "User '{}' exceeds daily cost budget: ${:.4} used of ${:.4}",
                        identity.user_id, current, max_daily_cost
                    )),
                });
            }
        }

        Ok(GovernanceResult {
            allowed: true,
            rules_evaluated,
            violation: None,
        })
    }

    pub async fn record_usage(&self, identity: &CallerIdentity, tokens: u64, cost: f64) {
        let usage = self.daily_usage.read().await;
        usage.tokens_by_user.entry(identity.user_id.clone()).and_modify(|v| *v += tokens).or_insert(tokens);
        usage.cost_by_user.entry(identity.user_id.clone()).and_modify(|v| *v += cost).or_insert(cost);
    }

    pub async fn update_config(&self, new_config: GovernanceConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }

    pub async fn reset_daily(&self) {
        let usage = self.daily_usage.read().await;
        usage.tokens_by_user.clear();
        usage.cost_by_user.clear();
    }
}
