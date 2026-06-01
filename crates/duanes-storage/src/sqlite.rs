use duanes_common::config::{PrivacyConfig, StorageConfig};
use duanes_common::errors::{DuanesError, DuanesResult};
use duanes_common::models::InferenceRecord;
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use crate::privacy::PrivacyFilter;

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
    privacy: PrivacyFilter,
}

impl SqliteStorage {
    pub fn new(config: &StorageConfig) -> DuanesResult<Self> {
        let conn = match &config.backend {
            duanes_common::config::StorageBackend::Sqlite => {
                let path = Path::new(&config.path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| DuanesError::Storage(format!("Failed to create db directory: {}", e)))?;
                }
                Connection::open(path)
                    .map_err(|e| DuanesError::Storage(format!("Failed to open database: {}", e)))?
            }
            duanes_common::config::StorageBackend::InMemory => {
                Connection::open_in_memory()
                    .map_err(|e| DuanesError::Storage(format!("Failed to create in-memory db: {}", e)))?
            }
        };

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA cache_size=-64000;")
            .map_err(|e| DuanesError::Storage(format!("Failed to set pragmas: {}", e)))?;

        let privacy = PrivacyFilter::new(config.privacy.clone());

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
            privacy,
        };

        storage.run_migrations()?;
        Ok(storage)
    }

    pub fn in_memory() -> DuanesResult<Self> {
        let config = StorageConfig {
            backend: duanes_common::config::StorageBackend::InMemory,
            path: ":memory:".to_string(),
            privacy: PrivacyConfig::default(),
        };
        Self::new(&config)
    }

    fn run_migrations(&self) -> DuanesResult<()> {
        let conn = self.conn.lock();
        crate::migration::run_migrations(&conn)?;
        Ok(())
    }

    pub async fn store(&self, record: &InferenceRecord) -> DuanesResult<()> {
        let filtered = self.privacy.apply(record);
        let conn = self.conn.lock();

        conn.execute(
            "INSERT INTO inference_records (
                id, request_id, timestamp, user_id, app_id, team_id, org_id,
                provider, model, execution_mode, prompt_hash, prompt_text,
                response_text, prompt_tokens, completion_tokens, total_tokens,
                time_to_first_token_ms, total_latency_ms, routing_overhead_ms,
                prompt_cost, completion_cost, total_cost, currency,
                governance_allowed, governance_rules, governance_violation,
                metadata
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27)",
            params![
                filtered.id.to_string(),
                filtered.request_id.to_string(),
                filtered.timestamp.to_rfc3339(),
                filtered.identity.as_ref().map(|i| &i.user_id),
                filtered.identity.as_ref().and_then(|i| i.app_id.as_ref()),
                filtered.identity.as_ref().and_then(|i| i.team_id.as_ref()),
                filtered.identity.as_ref().and_then(|i| i.org_id.as_ref()),
                filtered.provider,
                filtered.model,
                serde_json::to_string(&filtered.execution_mode).unwrap_or_default(),
                filtered.prompt_hash,
                filtered.prompt_text,
                filtered.response_text,
                filtered.usage.prompt_tokens,
                filtered.usage.completion_tokens,
                filtered.usage.total_tokens,
                filtered.latency.time_to_first_token_ms,
                filtered.latency.total_latency_ms,
                filtered.latency.routing_overhead_ms,
                filtered.cost.prompt_cost,
                filtered.cost.completion_cost,
                filtered.cost.total_cost,
                filtered.cost.currency,
                filtered.governance_result.allowed,
                serde_json::to_string(&filtered.governance_result.rules_evaluated).unwrap_or_default(),
                filtered.governance_result.violation,
                serde_json::to_string(&filtered.metadata).unwrap_or_default(),
            ],
        ).map_err(|e| DuanesError::Storage(format!("Failed to insert record: {}", e)))?;

        Ok(())
    }

    pub async fn query_recent(&self, limit: u64) -> DuanesResult<Vec<InferenceRecord>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT * FROM inference_records ORDER BY timestamp DESC LIMIT ?1"
        ).map_err(|e| DuanesError::Storage(e.to_string()))?;

        let records = stmt.query_map(params![limit], |row| {
            Ok(row_to_record(row))
        }).map_err(|e| DuanesError::Storage(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    pub async fn get_daily_summary(&self, user_id: Option<&str>) -> DuanesResult<DailySummary> {
        let conn = self.conn.lock();
        let query = if user_id.is_some() {
            "SELECT COUNT(*) as count, COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0.0), COALESCE(AVG(total_latency_ms), 0.0) FROM inference_records WHERE date(timestamp) = date('now') AND user_id = ?1"
        } else {
            "SELECT COUNT(*) as count, COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0.0), COALESCE(AVG(total_latency_ms), 0.0) FROM inference_records WHERE date(timestamp) = date('now')"
        };

        let mut stmt = conn.prepare(query).map_err(|e| DuanesError::Storage(e.to_string()))?;

        let summary = if let Some(uid) = user_id {
            stmt.query_row(params![uid], |row| {
                Ok(DailySummary {
                    total_requests: row.get(0)?,
                    total_tokens: row.get(1)?,
                    total_cost: row.get(2)?,
                    avg_latency_ms: row.get(3)?,
                })
            })
        } else {
            stmt.query_row([], |row| {
                Ok(DailySummary {
                    total_requests: row.get(0)?,
                    total_tokens: row.get(1)?,
                    total_cost: row.get(2)?,
                    avg_latency_ms: row.get(3)?,
                })
            })
        }.map_err(|e| DuanesError::Storage(e.to_string()))?;

        Ok(summary)
    }

    pub async fn get_pending_sync(&self, batch_size: usize) -> DuanesResult<Vec<InferenceRecord>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT * FROM inference_records WHERE synced = 0 ORDER BY timestamp ASC LIMIT ?1"
        ).map_err(|e| DuanesError::Storage(e.to_string()))?;

        let records = stmt.query_map(params![batch_size], |row| {
            Ok(row_to_record(row))
        }).map_err(|e| DuanesError::Storage(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    pub async fn mark_synced(&self, ids: &[uuid::Uuid]) -> DuanesResult<()> {
        let conn = self.conn.lock();
        for id in ids {
            conn.execute(
                "UPDATE inference_records SET synced = 1 WHERE id = ?1",
                params![id.to_string()],
            ).map_err(|e| DuanesError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DailySummary {
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub avg_latency_ms: f64,
}

fn row_to_record(row: &rusqlite::Row) -> InferenceRecord {
    InferenceRecord {
        id: uuid::Uuid::parse_str(row.get::<_, String>("id").unwrap_or_default().as_str()).unwrap_or_default(),
        request_id: uuid::Uuid::parse_str(row.get::<_, String>("request_id").unwrap_or_default().as_str()).unwrap_or_default(),
        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>("timestamp").unwrap_or_default())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_default(),
        identity: None,
        provider: row.get("provider").unwrap_or_default(),
        model: row.get("model").unwrap_or_default(),
        execution_mode: duanes_common::models::ExecutionMode::CloudProxy,
        prompt_hash: row.get("prompt_hash").unwrap_or(None),
        prompt_text: row.get("prompt_text").unwrap_or(None),
        response_text: row.get("response_text").unwrap_or(None),
        usage: duanes_common::models::TokenUsage {
            prompt_tokens: row.get("prompt_tokens").unwrap_or(0),
            completion_tokens: row.get("completion_tokens").unwrap_or(0),
            total_tokens: row.get("total_tokens").unwrap_or(0),
        },
        latency: duanes_common::models::LatencyMetrics {
            time_to_first_token_ms: row.get("time_to_first_token_ms").unwrap_or(None),
            total_latency_ms: row.get("total_latency_ms").unwrap_or(0),
            routing_overhead_ms: row.get("routing_overhead_ms").unwrap_or(0),
        },
        cost: duanes_common::models::CostMetrics {
            prompt_cost: row.get("prompt_cost").unwrap_or(0.0),
            completion_cost: row.get("completion_cost").unwrap_or(0.0),
            total_cost: row.get("total_cost").unwrap_or(0.0),
            currency: row.get("currency").unwrap_or("USD".to_string()),
        },
        governance_result: duanes_common::models::GovernanceResult {
            allowed: row.get("governance_allowed").unwrap_or(true),
            rules_evaluated: serde_json::from_str(&row.get::<_, String>("governance_rules").unwrap_or_default()).unwrap_or_default(),
            violation: row.get("governance_violation").unwrap_or(None),
        },
        metadata: serde_json::from_str(&row.get::<_, String>("metadata").unwrap_or_default()).unwrap_or_default(),
    }
}
