use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{Duration, interval};

pub struct AggregatorWorker {
    db: Arc<Mutex<Connection>>,
    interval_secs: u64,
}

impl AggregatorWorker {
    pub fn new(db: Arc<Mutex<Connection>>, interval_secs: u64) -> Self {
        Self { db, interval_secs }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut ticker = interval(Duration::from_secs(self.interval_secs));

        loop {
            ticker.tick().await;
            self.aggregate().await?;
        }
    }

    async fn aggregate(&self) -> anyhow::Result<()> {
        let conn = self.db.lock().unwrap();

        conn.execute_batch(r#"
            INSERT OR REPLACE INTO daily_aggregates (date, provider, model, user_id, source_id, total_requests, total_tokens, total_cost, avg_latency_ms)
            SELECT
                date(timestamp),
                provider,
                model,
                COALESCE(user_id, '__all__'),
                COALESCE(source_id, '__all__'),
                COUNT(*),
                SUM(total_tokens),
                SUM(total_cost),
                AVG(total_latency_ms)
            FROM collected_records
            WHERE date(timestamp) = date('now')
            GROUP BY date(timestamp), provider, model, user_id;
        "#)?;

        tracing::info!("Completed daily aggregation cycle");
        Ok(())
    }
}
