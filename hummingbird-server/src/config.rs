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

    /// OIDC client_id for Authorization Code flow.
    /// When set, enables browser login via OIDC and disables local password login.
    #[arg(long)]
    pub oidc_client_id: Option<String>,

    /// OIDC client_secret for confidential clients.
    #[arg(long)]
    pub oidc_client_secret: Option<String>,

    /// Base URL of this app (e.g. https://music.example.com).
    /// Required when --oidc-client-id is set, used to build the callback URL.
    #[arg(long)]
    pub public_url: Option<String>,

    /// OIDC claim to use for role mapping (default: "groups").
    /// Supports dot-notation for nested claims (e.g. "realm_access.roles").
    #[arg(long, default_value = "groups")]
    pub oidc_role_claim: String,

    /// Value in the role claim that maps to the admin role.
    #[arg(long)]
    pub oidc_admin_group: Option<String>,
}
