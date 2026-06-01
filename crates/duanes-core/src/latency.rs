use duanes_common::models::{CostMetrics, InferenceRequest, InferenceResponse, LatencyMetrics, ModelPricing, TokenUsage};
use std::time::Instant;

pub struct LatencyTracer {
    routing_start: Instant,
    request_start: Instant,
    first_token_at: Option<Instant>,
}

impl LatencyTracer {
    pub fn start() -> Self {
        let now = Instant::now();
        Self {
            routing_start: now,
            request_start: now,
            first_token_at: None,
        }
    }

    pub fn mark_routing_complete(&mut self) {
        self.request_start = Instant::now();
    }

    pub fn mark_first_token(&mut self) {
        if self.first_token_at.is_none() {
            self.first_token_at = Some(Instant::now());
        }
    }

    pub fn compute_metrics(&self) -> LatencyMetrics {
        let total_latency_ms = self.routing_start.elapsed().as_millis() as u64;
        let routing_overhead_ms = self.request_start
            .duration_since(self.routing_start)
            .as_millis() as u64;
        let time_to_first_token_ms = self.first_token_at
            .map(|t| t.duration_since(self.request_start).as_millis() as u64);

        LatencyMetrics {
            time_to_first_token_ms,
            total_latency_ms,
            routing_overhead_ms,
        }
    }
}

pub struct CostCalculator;

impl CostCalculator {
    pub fn calculate(usage: &TokenUsage, pricing: &ModelPricing) -> CostMetrics {
        let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * pricing.prompt_cost_per_million;
        let completion_cost = (usage.completion_tokens as f64 / 1_000_000.0) * pricing.completion_cost_per_million;
        let total_cost = prompt_cost + completion_cost;

        CostMetrics {
            prompt_cost,
            completion_cost,
            total_cost,
            currency: "USD".to_string(),
        }
    }

    pub fn estimate(request: &InferenceRequest, pricing: &ModelPricing) -> CostMetrics {
        let prompt_tokens = crate::openai::OpenAICompatLayer::count_prompt_tokens(request);
        let estimated_completion = request.max_tokens.unwrap_or(prompt_tokens);

        CostCalculator::calculate(
            &TokenUsage {
                prompt_tokens,
                completion_tokens: estimated_completion,
                total_tokens: prompt_tokens + estimated_completion,
            },
            pricing,
        )
    }
}
