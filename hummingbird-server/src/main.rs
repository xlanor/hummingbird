use std::sync::Arc;

use clap::Parser;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hummingbird_server::config::Config;
use hummingbird_server::db;
use hummingbird_server::db::sqlite::SqliteRepository;
use hummingbird_server::routes::{self, AppState};
use hummingbird_server::scanner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::parse();

    // Parse database URL
    let db_url = &config.db;
    let pool = if db_url.starts_with("sqlite:") {
        let path = db_url.strip_prefix("sqlite://").unwrap_or(
            db_url.strip_prefix("sqlite:").unwrap_or(db_url),
        );

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?
    } else {
        anyhow::bail!(
            "unsupported database URL: {db_url}. Only sqlite:// is supported in this version."
        );
    };

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await?;
    let repo: Arc<dyn db::Repository> = Arc::new(repo);

    let scan_handle = scanner::start_scanner(repo.clone(), config.scan_dir.clone());

    let state = Arc::new(AppState { repo, scan_handle });

    let app = routes::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    info!("listening on {}", config.bind);

    axum::serve(listener, app).await?;

    Ok(())
}
