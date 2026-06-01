use clap::Parser;
use std::sync::Arc;
use std::sync::Mutex;
use tracing_subscriber::EnvFilter;

mod aggregator;
mod retention;

#[derive(Parser, Debug)]
#[command(name = "duanes-telemetry-processor", about = "Duanes Telemetry Aggregation Worker")]
struct Args {
    #[arg(long, default_value = "collector.db")]
    db_path: String,

    #[arg(long, default_value_t = 60)]
    aggregation_interval_secs: u64,

    #[arg(long, default_value_t = 90)]
    retention_days: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("duanes=info".parse()?))
        .init();

    let args = Args::parse();

    tracing::info!(
        db = %args.db_path,
        interval_secs = args.aggregation_interval_secs,
        retention_days = args.retention_days,
        "Starting telemetry processor"
    );

    let conn = rusqlite::Connection::open(&args.db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;

    let db = Arc::new(Mutex::new(conn));

    let agg = Arc::new(aggregator::AggregatorWorker::new(db.clone(), args.aggregation_interval_secs));
    let ret = Arc::new(retention::RetentionWorker::new(db.clone(), args.retention_days));

    let agg_clone = agg.clone();
    let ret_clone = ret.clone();

    let agg_handle = tokio::spawn(async move { agg_clone.run().await });
    let ret_handle = tokio::spawn(async move { ret_clone.run().await });

    tokio::select! {
        r = agg_handle => {
            if let Err(e) = r {
                tracing::error!(error = %e, "Aggregator worker panicked");
            }
        }
        r = ret_handle => {
            if let Err(e) = r {
                tracing::error!(error = %e, "Retention worker panicked");
            }
        }
    }

    Ok(())
}
