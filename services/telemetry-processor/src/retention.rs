use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{Duration, interval};

pub struct RetentionWorker {
    db: Arc<Mutex<Connection>>,
    retention_days: u32,
}

impl RetentionWorker {
    pub fn new(db: Arc<Mutex<Connection>>, retention_days: u32) -> Self {
        Self { db, retention_days }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut ticker = interval(Duration::from_secs(3600));

        loop {
            ticker.tick().await;
            self.cleanup().await?;
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        let conn = self.db.lock().unwrap();
        let days = self.retention_days;

        let deleted = conn.execute(
            "DELETE FROM collected_records WHERE date(timestamp) < date('now', ?1)",
            rusqlite::params![format!("-{} days", days)],
        )?;

        if deleted > 0 {
            tracing::info!(deleted_rows = deleted, retention_days = days, "Cleaned up old records");
        }

        Ok(())
    }
}
