use std::sync::Arc;

use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hummingbird_server::config::Config;
use hummingbird_server::db::{self, Repository};
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
    let db_url = &config.db;

    let repo: Arc<dyn Repository> = if db_url.starts_with("sqlite:") {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let path = db_url
            .strip_prefix("sqlite://")
            .unwrap_or(db_url.strip_prefix("sqlite:").unwrap_or(db_url));

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let repo = db::sqlite::SqliteRepository::new(pool);
        repo.run_migrations().await?;
        info!("connected to SQLite database");
        Arc::new(repo)
    } else if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        use sqlx::postgres::PgPoolOptions;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(db_url)
            .await?;

        let repo = db::postgres::PostgresRepository::new(pool);
        repo.run_migrations().await?;
        info!("connected to PostgreSQL database");
        Arc::new(repo)
    } else if db_url.starts_with("mysql://") || db_url.starts_with("mariadb://") {
        use sqlx::mysql::MySqlPoolOptions;

        let connect_url = if db_url.starts_with("mariadb://") {
            db_url.replacen("mariadb://", "mysql://", 1)
        } else {
            db_url.clone()
        };

        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect(&connect_url)
            .await?;

        let repo = db::mariadb::MariaDbRepository::new(pool);
        repo.run_migrations().await?;
        info!("connected to MariaDB/MySQL database");
        Arc::new(repo)
    } else {
        anyhow::bail!(
            "unsupported database URL: {db_url}. \
             Supported prefixes: sqlite://, postgres://, postgresql://, mysql://, mariadb://"
        );
    };

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
