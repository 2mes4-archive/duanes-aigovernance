use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::*;
use crate::governance::GovernanceEngine;
use crate::latency::{CostCalculator, LatencyTracer};
use crate::provider::{ProviderRef, ProviderRegistry};
use crate::fallback::FallbackManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct InferenceRouter {
    registry: Arc<ProviderRegistry>,
    governance: Arc<GovernanceEngine>,
    fallback: Arc<FallbackManager>,
    default_provider: Arc<RwLock<String>>,
    local_runner: Option<Arc<dyn LocalInferenceRunner>>,
}

#[async_trait::async_trait]
pub trait LocalInferenceRunner: Send + Sync {
    async fn inference(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse>;
    fn supports_model(&self, model: &str) -> bool;
    fn name(&self) -> &str;
}

pub struct RouterResult {
    pub response: InferenceResponse,
    pub record: InferenceRecord,
}

impl InferenceRouter {
    pub fn new(
        registry: Arc<ProviderRegistry>,
        governance: Arc<GovernanceEngine>,
        default_provider: String,
    ) -> Self {
        Self {
            registry,
            governance,
            fallback: Arc::new(FallbackManager::new()),
            default_provider: Arc::new(RwLock::new(default_provider)),
            local_runner: None,
        }
    }

    pub fn with_local_runner(mut self, runner: Arc<dyn LocalInferenceRunner>) -> Self {
        self.local_runner = Some(runner);
        self
    }

    pub async fn route(
        &self,
        request: InferenceRequest,
        identity: Option<CallerIdentity>,
    ) -> DuanesResult<RouterResult> {
        let mut tracer = LatencyTracer::start();

        let governance_result = self.governance
            .evaluate(&request, identity.as_ref())
            .await?;

        if !governance_result.allowed {
            return Err(DuanesError::GovernanceViolation(
                governance_result.violation.unwrap_or_default()
            ));
        }

        tracer.mark_routing_complete();

        let execution_mode;
        let provider_name;

        let response = if self.should_run_locally(&request) {
            execution_mode = ExecutionMode::LocalInference;
            provider_name = "local".to_string();
            self.run_local(request.clone()).await?
        } else {
            let provider = self.select_provider(&request).await?;
            provider_name = provider.name().to_string();

            match provider.inference(request.clone()).await {
                Ok(response) => {
                    execution_mode = ExecutionMode::CloudProxy;
                    response
                }
                Err(e) => {
                    tracing::warn!(provider = %provider_name, error = %e, "Primary provider failed, attempting fallback");
                    match self.attempt_fallback(request.clone()).await {
                        Some(fallback_response) => {
                            execution_mode = ExecutionMode::Fallback;
                            fallback_response
                        }
                        None => return Err(e),
                    }
                }
            }
        };

        tracer.mark_first_token();

        let latency = tracer.compute_metrics();
        let prompt_tokens = crate::openai::OpenAICompatLayer::count_prompt_tokens(&request);
        let completion_tokens = crate::openai::OpenAICompatLayer::count_completion_tokens(
            &response.choices.first()
                .map(|c| c.message.content.text())
                .unwrap_or_default()
        );

        let usage = TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        };

        let model_pricing = ModelPricing {
            model_id: request.model.clone(),
            prompt_cost_per_million: 0.0,
            completion_cost_per_million: 0.0,
        };
        let cost = CostCalculator::calculate(&usage, &model_pricing);

        if let Some(ref id) = identity {
            self.governance.record_usage(id, usage.total_tokens, cost.total_cost).await;
        }

        let prompt_text = request.messages.iter()
            .map(|m| m.content.text())
            .collect::<Vec<_>>()
            .join("\n");

        let response_text = response.choices.first()
            .map(|c| c.message.content.text())
            .unwrap_or_default();

        let record = InferenceRecord {
            id: Uuid::new_v4(),
            request_id: request.id,
            timestamp: chrono::Utc::now(),
            identity,
            provider: provider_name,
            model: request.model.clone(),
            execution_mode,
            prompt_hash: Some(self.hash_text(&prompt_text)),
            prompt_text: Some(prompt_text),
            response_text: Some(response_text),
            usage,
            latency,
            cost,
            governance_result,
            metadata: serde_json::Map::new(),
        };

        Ok(RouterResult { response, record })
    }

    fn should_run_locally(&self, request: &InferenceRequest) -> bool {
        if let Some(ref runner) = self.local_runner {
            return runner.supports_model(&request.model);
        }
        false
    }

    async fn run_local(&self, request: InferenceRequest) -> DuanesResult<InferenceResponse> {
        let runner = self.local_runner.as_ref()
            .ok_or_else(|| DuanesError::LocalInference("No local runner configured".to_string()))?;
        runner.inference(request).await
    }

    async fn select_provider(&self, request: &InferenceRequest) -> DuanesResult<ProviderRef> {
        let default = self.default_provider.read().await.clone();
        if let Some(provider) = self.registry.get(&default).await {
            if provider.supports_model(&request.model) {
                return Ok(provider);
            }
        }

        let providers = self.registry.list_providers().await;
        for name in providers {
            if let Some(provider) = self.registry.get(&name).await {
                if provider.supports_model(&request.model) {
                    return Ok(provider);
                }
            }
        }

        self.registry.get(&default).await
            .ok_or_else(|| DuanesError::ProviderUnavailable(format!("No provider found for model '{}'", request.model)))
    }

    async fn attempt_fallback(&self, request: InferenceRequest) -> Option<InferenceResponse> {
        if let Some(ref runner) = self.local_runner {
            if runner.supports_model(&request.model) {
                tracing::info!("Falling back to local inference");
                return runner.inference(request).await.ok();
            }
        }
        None
    }

    fn hash_text(&self, text: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub async fn set_default_provider(&self, name: String) {
        let mut default = self.default_provider.write().await;
        *default = name;
    }
}
