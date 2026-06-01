use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

pub struct Aggregator {
    db: Arc<Mutex<Connection>>,
}

impl Aggregator {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    pub fn get_summary(&self, group_by: &str, since: Option<&str>) -> anyhow::Result<serde_json::Value> {
        let conn = self.db.lock();
        let query = match group_by {
            "provider" => "SELECT provider, COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records GROUP BY provider".to_string(),
            "model" => "SELECT model, COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records GROUP BY model".to_string(),
            "user" => "SELECT user_id, COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records WHERE user_id IS NOT NULL GROUP BY user_id".to_string(),
            "source" => "SELECT source_id, COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records GROUP BY source_id".to_string(),
            "day" => "SELECT date(timestamp), COUNT(*), SUM(total_tokens), SUM(total_cost), AVG(total_latency_ms) FROM collected_records GROUP BY date(timestamp) ORDER BY date(timestamp) DESC".to_string(),
            _ => return Ok(serde_json::json!({"error": "Invalid group_by. Use: provider, model, user, source, day"})),
        };

        let mut stmt = conn.prepare(&query)?;
        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "total_requests": row.get::<_, i64>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "total_cost": row.get::<_, f64>(3)?,
                "avg_latency_ms": row.get::<_, f64>(4)?,
            }))
        })?.filter_map(|r| r.ok()).collect();

        Ok(serde_json::json!({ "data": rows }))
    }

    pub fn get_peak_usage(&self) -> anyhow::Result<serde_json::Value> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d %H:00', timestamp) as hour, COUNT(*) as requests, SUM(total_cost) as cost FROM collected_records GROUP BY hour ORDER BY requests DESC LIMIT 10"
        )?;

        let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "hour": row.get::<_, String>(0)?,
                "requests": row.get::<_, i64>(1)?,
                "cost": row.get::<_, f64>(2)?,
            }))
        })?.filter_map(|r| r.ok()).collect();

        Ok(serde_json::json!({ "peak_hours": rows }))
    }
}
