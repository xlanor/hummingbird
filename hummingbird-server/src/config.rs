use camino::Utf8PathBuf;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "hummingbird-server", about = "Hummingbird Music Server")]
pub struct Config {
    /// Database URL. Prefix determines backend:
    ///   sqlite:///path/to/library.db
    ///   postgres://user:pass@host/dbname
    ///   mysql://user:pass@host/dbname  (MariaDB)
    #[arg(long)]
    pub db: String,

    /// Directory to scan for music files (can be specified multiple times)
    #[arg(long)]
    pub scan_dir: Vec<Utf8PathBuf>,

    /// Address to bind the server to
    #[arg(long, default_value = "0.0.0.0:3000")]
    pub bind: String,

    /// Secret key for signing local JWT tokens (min 32 chars).
    /// If not set, a random key is generated on each startup.
    #[arg(long)]
    pub jwt_secret: Option<String>,

    /// OIDC issuer URL (e.g., https://accounts.google.com).
    /// When set, the server accepts OIDC Bearer tokens.
    #[arg(long)]
    pub oidc_issuer: Option<String>,

    /// OIDC audience (client_id) for token validation.
    #[arg(long)]
    pub oidc_audience: Option<String>,
}
