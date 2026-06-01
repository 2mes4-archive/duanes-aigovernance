use rusqlite::Connection;
use duanes_common::errors::{DuanesError, DuanesResult};

pub fn run_migrations(conn: &Connection) -> DuanesResult<()> {
    let current_version: i32 = conn
        .query_row(
            "SELECT value FROM schema_version WHERE key = 'version'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let migrations = get_migrations();

    for (version, sql) in migrations.iter() {
        if *version > current_version {
            conn.execute_batch(sql)
                .map_err(|e| DuanesError::Storage(format!("Migration {} failed: {}", version, e)))?;
        }
    }

    let max_version = migrations.last().map(|(v, _)| *v).unwrap_or(0);
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (key TEXT PRIMARY KEY, value INTEGER)",
        [],
    ).map_err(|e| DuanesError::Storage(e.to_string()))?;

    conn.execute(
        "INSERT OR REPLACE INTO schema_version (key, value) VALUES ('version', ?1)",
        rusqlite::params![max_version],
    ).map_err(|e| DuanesError::Storage(e.to_string()))?;

    Ok(())
}

fn get_migrations() -> Vec<(i32, &'static str)> {
    vec![
        (1, r#"
            CREATE TABLE IF NOT EXISTS inference_records (
                id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                user_id TEXT,
                app_id TEXT,
                team_id TEXT,
                org_id TEXT,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                execution_mode TEXT NOT NULL,
                prompt_hash TEXT,
                prompt_text TEXT,
                response_text TEXT,
                prompt_tokens INTEGER NOT NULL DEFAULT 0,
                completion_tokens INTEGER NOT NULL DEFAULT 0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                time_to_first_token_ms INTEGER,
                total_latency_ms INTEGER NOT NULL DEFAULT 0,
                routing_overhead_ms INTEGER NOT NULL DEFAULT 0,
                prompt_cost REAL NOT NULL DEFAULT 0.0,
                completion_cost REAL NOT NULL DEFAULT 0.0,
                total_cost REAL NOT NULL DEFAULT 0.0,
                currency TEXT NOT NULL DEFAULT 'USD',
                governance_allowed BOOLEAN NOT NULL DEFAULT 1,
                governance_rules TEXT,
                governance_violation TEXT,
                metadata TEXT,
                synced BOOLEAN NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_records_timestamp ON inference_records(timestamp);
            CREATE INDEX IF NOT EXISTS idx_records_user_id ON inference_records(user_id);
            CREATE INDEX IF NOT EXISTS idx_records_provider ON inference_records(provider);
            CREATE INDEX IF NOT EXISTS idx_records_model ON inference_records(model);
            CREATE INDEX IF NOT EXISTS idx_records_synced ON inference_records(synced);
            CREATE INDEX IF NOT EXISTS idx_records_date ON inference_records(date(timestamp));
        "#),
    ]
}
