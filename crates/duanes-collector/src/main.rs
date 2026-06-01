use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod ingestion;
mod aggregation;

#[derive(Parser, Debug)]
#[command(name = "duanes-collector", about = "Duanes Central Telemetry Collector")]
struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value_t = 9090)]
    port: u16,

    #[arg(long, default_value = "collector.db")]
    db_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("duanes=info".parse()?))
        .json()
        .init();

    let args = Args::parse();

    tracing::info!(host = %args.host, port = %args.port, "Starting Duanes collector");

    let conn = rusqlite::Connection::open(&args.db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS collected_records (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL,
            batch_id TEXT NOT NULL,
            received_at TEXT NOT NULL,
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
            prompt_tokens INTEGER DEFAULT 0,
            completion_tokens INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            time_to_first_token_ms INTEGER,
            total_latency_ms INTEGER DEFAULT 0,
            routing_overhead_ms INTEGER DEFAULT 0,
            prompt_cost REAL DEFAULT 0.0,
            completion_cost REAL DEFAULT 0.0,
            total_cost REAL DEFAULT 0.0,
            currency TEXT DEFAULT 'USD',
            governance_allowed BOOLEAN DEFAULT 1,
            metadata TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_collected_source ON collected_records(source_id);
        CREATE INDEX IF NOT EXISTS idx_collected_timestamp ON collected_records(timestamp);
        CREATE INDEX IF NOT EXISTS idx_collected_user ON collected_records(user_id);
    "#)?;

    let db = Arc::new(parking_lot::Mutex::new(conn));
    let agg = Arc::new(aggregation::Aggregator::new(db.clone()));

    let app_state = ingestion::CollectorState {
        db,
        aggregator: agg,
    };

    let app = ingestion::create_router(app_state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "Duanes collector listening");

    axum::serve(listener, app).await?;

    Ok(())
}
