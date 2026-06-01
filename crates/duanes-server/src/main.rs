use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

mod routes;
mod middleware;
mod identity;

#[derive(Parser, Debug)]
#[command(name = "duanes-server", about = "Duanes AI Governance Proxy Server")]
struct Args {
    #[arg(short, long, default_value = "config/duanes.toml")]
    config: String,

    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("duanes=info".parse()?))
        .json()
        .init();

    let args = Args::parse();

    tracing::info!(config = %args.config, host = %args.host, port = %args.port, "Starting Duanes server");

    let config = duanes_common::DuanesConfig::from_file(&args.config)
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    let storage = Arc::new(
        duanes_storage::SqliteStorage::new(&config.storage)?
    );

    let provider_registry = Arc::new(duanes_core::ProviderRegistry::new());

    let governance = Arc::new(
        duanes_core::GovernanceEngine::new(config.governance.clone())
    );

    let router = Arc::new(
        duanes_core::InferenceRouter::new(
            provider_registry.clone(),
            governance.clone(),
            config.engine.default_provider.clone(),
        )
    );

    let collector_sync = if let Some(ref collector_config) = config.collector {
        let source_id = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let sync = Arc::new(
            duanes_storage::CollectorSync::new(collector_config.clone(), source_id)
        );

        let sync_clone = sync.clone();
        let storage_clone = storage.clone();
        tokio::spawn(async move {
            sync_clone.start(storage_clone).await;
        });

        Some(sync)
    } else {
        None
    };

    let app_state = routes::AppState {
        router,
        storage,
        governance,
        provider_registry,
        config: Arc::new(config),
    };

    let app = routes::create_router(app_state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "Duanes server listening");

    axum::serve(listener, app).await?;

    Ok(())
}
