use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use duanes_common::models::SyncPayload;
use parking_lot::Mutex;
use rusqlite::Connection;
use serde_json::Value;
use std::sync::Arc;

pub struct CollectorState {
    pub db: Arc<Mutex<Connection>>,
    pub aggregator: Arc<crate::aggregation::Aggregator>,
}

pub fn create_router(state: CollectorState) -> Router {
    Router::new()
        .route("/api/v1/ingest", post(ingest))
        .route("/api/v1/stats/overview", get(stats_overview))
        .route("/api/v1/stats/by-source", get(stats_by_source))
        .route("/api/v1/stats/by-user", get(stats_by_user))
        .route("/api/v1/stats/by-model", get(stats_by_model))
        .route("/health", get(health))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

async fn ingest(
    State(state): State<Arc<CollectorState>>,
    Json(payload): Json<SyncPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock();
    let received_at = chrono::Utc::now().to_rfc3339();

    for record in &payload.records {
        let identity = record.identity.as_ref();
        conn.execute(
            "INSERT OR IGNORE INTO collected_records (
                id, source_id, batch_id, received_at, request_id, timestamp,
                user_id, app_id, team_id, org_id, provider, model, execution_mode,
                prompt_hash, prompt_tokens, completion_tokens, total_tokens,
                time_to_first_token_ms, total_latency_ms, routing_overhead_ms,
                prompt_cost, completion_cost, total_cost, currency,
                governance_allowed, metadata
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
            rusqlite::params![
                record.id.to_string(),
                payload.source_id,
                payload.batch_id.to_string(),
                received_at,
                record.request_id.to_string(),
                record.timestamp.to_rfc3339(),
                identity.map(|i| &i.user_id),
                identity.and_then(|i| i.app_id.as_ref()),
                identity.and_then(|i| i.team_id.as_ref()),
                identity.and_then(|i| i.org_id.as_ref()),
                record.provider,
                record.model,
                serde_json::to_string(&record.execution_mode).unwrap_or_default(),
                record.prompt_hash,
                record.usage.prompt_tokens,
                record.usage.completion_tokens,
                record.usage.total_tokens,
                record.latency.time_to_first_token_ms,
                record.latency.total_latency_ms,
                record.latency.routing_overhead_ms,
                record.cost.prompt_cost,
                record.cost.completion_cost,
                record.cost.total_cost,
                record.cost.currency,
                record.governance_result.allowed,
                serde_json::to_string(&record.metadata).unwrap_or_default(),
            ],
        ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    tracing::info!(
        source = %payload.source_id,
        batch = %payload.batch_id,
        records = payload.records.len(),
        "Ingested records"
    );

    Ok(StatusCode::ACCEPTED)
}

async fn stats_overview(
    State(state): State<Arc<CollectorState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let conn = state.db.lock();
    let stats: (i64, i64, f64, f64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(total_tokens),0), COALESCE(SUM(total_cost),0.0), COALESCE(AVG(total_latency_ms),0.0) FROM collected_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "total_requests": stats.0,
        "total_tokens": stats.1,
        "total_cost": stats.2,
        "avg_latency_ms": stats.3,
    })))
}

async fn stats_by_source(
    State(state): State<Arc<CollectorState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare("SELECT source_id, COUNT(*), SUM(total_tokens), SUM(total_cost) FROM collected_records GROUP BY source_id")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "source_id": row.get::<_, String>(0)?,
                "total_requests": row.get::<_, i64>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "total_cost": row.get::<_, f64>(3)?,
            }))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(serde_json::json!({ "sources": rows })))
}

async fn stats_by_user(
    State(state): State<Arc<CollectorState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare("SELECT user_id, COUNT(*), SUM(total_tokens), SUM(total_cost) FROM collected_records WHERE user_id IS NOT NULL GROUP BY user_id ORDER BY SUM(total_cost) DESC LIMIT 100")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "user_id": row.get::<_, String>(0)?,
                "total_requests": row.get::<_, i64>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "total_cost": row.get::<_, f64>(3)?,
            }))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(serde_json::json!({ "users": rows })))
}

async fn stats_by_model(
    State(state): State<Arc<CollectorState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let conn = state.db.lock();
    let mut stmt = conn
        .prepare("SELECT model, COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records GROUP BY model ORDER BY SUM(total_cost) DESC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "model": row.get::<_, String>(0)?,
                "total_requests": row.get::<_, i64>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "total_cost": row.get::<_, f64>(3)?,
                "avg_latency_ms": row.get::<_, f64>(4)?,
            }))
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(serde_json::json!({ "models": rows })))
}

async fn health() -> &'static str {
    "ok"
}
