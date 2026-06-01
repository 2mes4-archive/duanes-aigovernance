use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use duanes_common::models::{InferenceRequest, InferenceResponse};
use serde_json::Value;
use std::sync::Arc;
use crate::identity::extract_identity;

pub struct AppState {
    pub router: Arc<duanes_core::InferenceRouter>,
    pub storage: Arc<duanes_storage::SqliteStorage>,
    pub governance: Arc<duanes_core::GovernanceEngine>,
    pub provider_registry: Arc<duanes_core::ProviderRegistry>,
    pub config: Arc<duanes_common::DuanesConfig>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health))
        .route("/api/v1/stats/daily", get(daily_stats))
        .route("/api/v1/stats/recent", get(recent_records))
        .route("/api/v1/providers", get(list_providers))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(mut request): Json<Value>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, String)> {
    let identity = extract_identity(&headers);

    if let Some(obj) = request.as_object_mut() {
        obj.remove("id");
    }

    let inference_request: InferenceRequest = match serde_json::from_value(request) {
        Ok(r) => r,
        Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Invalid request: {}", e))),
    };

    let result = state.router.route(inference_request, identity).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Err(e) = state.storage.store(&result.record).await {
        tracing::error!(error = %e, "Failed to store inference record");
    }

    let response_json = serde_json::to_value(&result.response)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::OK, Json(response_json)))
}

async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let providers = state.provider_registry.list_providers().await;
    Ok(Json(serde_json::json!({
        "object": "list",
        "data": providers.iter().map(|p| serde_json::json!({
            "id": p,
            "object": "model",
            "owned_by": "duanes"
        })).collect::<Vec<_>>()
    })))
}

async fn health() -> &'static str {
    "ok"
}

async fn daily_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let summary = state.storage.get_daily_summary(None).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "total_requests": summary.total_requests,
        "total_tokens": summary.total_tokens,
        "total_cost": summary.total_cost,
        "avg_latency_ms": summary.avg_latency_ms,
    })))
}

async fn recent_records(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let records = state.storage.query_recent(50).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::to_value(&records).unwrap_or_default()))
}

async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let providers = state.provider_registry.list_providers().await;
    Ok(Json(serde_json::json!({ "providers": providers })))
}
