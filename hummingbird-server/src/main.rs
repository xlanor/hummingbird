use std::sync::Arc;

use clap::Parser;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use hummingbird_server::api::{self, AppState};
use hummingbird_server::config::Config;
use hummingbird_server::domain::scanner::orchestrator;
use hummingbird_server::infrastructure::auth;
use hummingbird_server::infrastructure::persistence::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::parse();
    let db_url = &config.db;

    let db: Arc<dyn Database> = if db_url.starts_with("sqlite:") {
        use hummingbird_server::infrastructure::persistence::sqlite::SqliteDatabase;
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

        let db = SqliteDatabase::new(pool);
        db.run_migrations().await?;
        info!("connected to SQLite database");
        Arc::new(db)
    } else if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        use hummingbird_server::infrastructure::persistence::postgres::PostgresDatabase;
        use sqlx::postgres::PgPoolOptions;

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(db_url)
            .await?;

        let db = PostgresDatabase::new(pool);
        db.run_migrations().await?;
        info!("connected to PostgreSQL database");
        Arc::new(db)
    } else if db_url.starts_with("mysql://") || db_url.starts_with("mariadb://") {
        use hummingbird_server::infrastructure::persistence::mariadb::MariaDbDatabase;
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

        let db = MariaDbDatabase::new(pool);
        db.run_migrations().await?;
        info!("connected to MariaDB/MySQL database");
        Arc::new(db)
    } else {
        anyhow::bail!(
            "unsupported database URL: {db_url}. \
             Supported prefixes: sqlite://, postgres://, postgresql://, mysql://, mariadb://"
        );
    };

    let jwt_secret = match config.jwt_secret {
        Some(ref s) => s.as_bytes().to_vec(),
        None => {
            let mut secret = vec![0u8; 64];
            getrandom::fill(&mut secret).expect("failed to generate random JWT secret");
            info!("no --jwt-secret provided, generated random key (tokens won't survive restarts)");
            secret
        }
    };

    let oidc = match (&config.oidc_issuer, &config.oidc_audience) {
        (Some(issuer), Some(audience)) => {
            info!("discovering OIDC provider at {issuer}");
            let oidc_config = auth::discover_oidc(issuer, audience).await?;
            info!("OIDC discovery complete");
            Some(oidc_config)
        }
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("both --oidc-issuer and --oidc-audience must be set together");
        }
        _ => None,
    };

    ensure_admin_exists(&db).await?;

    let scan_handle = orchestrator::start_scanner(db.clone(), config.scan_dir.clone());

    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret,
        oidc,
    });

    let app = api::router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    info!("listening on {}", config.bind);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn ensure_admin_exists(db: &Arc<dyn Database>) -> anyhow::Result<()> {
    let users = db.list_users().await?;
    if users.is_empty() {
        let hash = auth::hash_password("admin")?;
        db.create_user("admin", Some("Administrator"), Some(&hash), "admin")
            .await?;
        info!("created default admin user (username: admin, password: admin) — change this immediately");
    }
    Ok(())
}
