use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "hummingbird-server", about = "Hummingbird Music Server")]
struct CliArgs {
    /// Path to a TOML config file
    #[arg(long)]
    config: Option<String>,

    /// Database URL (sqlite:// | postgres:// | mysql://)
    #[arg(long)]
    db: Option<String>,

    /// Directory to scan for music files (can be specified multiple times)
    #[arg(long)]
    scan_dir: Vec<Utf8PathBuf>,

    /// Address to bind the server to
    #[arg(long)]
    bind: Option<String>,

    /// Secret key for signing local JWT tokens (min 32 chars)
    #[arg(long)]
    jwt_secret: Option<String>,

    /// OIDC issuer URL
    #[arg(long)]
    oidc_issuer: Option<String>,

    /// OIDC audience for token validation
    #[arg(long)]
    oidc_audience: Option<String>,

    /// OIDC client_id for Authorization Code flow
    #[arg(long)]
    oidc_client_id: Option<String>,

    /// OIDC client_secret for confidential clients
    #[arg(long)]
    oidc_client_secret: Option<String>,

    /// Base URL of this app (e.g. https://music.example.com)
    #[arg(long)]
    public_url: Option<String>,

    /// OIDC claim to use for role mapping
    #[arg(long)]
    oidc_role_claim: Option<String>,

    /// Value in the role claim that maps to the admin role
    #[arg(long)]
    oidc_admin_group: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    db: Option<String>,
    scan_dirs: Option<Vec<Utf8PathBuf>>,
    #[serde(default)]
    server: ServerFileConfig,
    #[serde(default)]
    oidc: OidcFileConfig,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct ServerFileConfig {
    bind: Option<String>,
    jwt_secret: Option<String>,
    public_url: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct OidcFileConfig {
    issuer: Option<String>,
    audience: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    role_claim: Option<String>,
    admin_group: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub db: String,
    pub scan_dir: Vec<Utf8PathBuf>,
    pub bind: String,
    pub jwt_secret: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_audience: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub public_url: Option<String>,
    pub oidc_role_claim: String,
    pub oidc_admin_group: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let cli = CliArgs::parse();
        let file = match &cli.config {
            Some(path) => {
                let content = std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read config file: {path}"))?;
                toml::from_str(&content)
                    .with_context(|| format!("failed to parse config file: {path}"))?
            }
            None => FileConfig::default(),
        };
        merge(cli, file)
    }
}

fn merge(cli: CliArgs, file: FileConfig) -> Result<Config> {
    let db = cli
        .db
        .or(file.db)
        .ok_or_else(|| anyhow::anyhow!("database URL is required (--db or db in config file)"))?;

    let scan_dir = if !cli.scan_dir.is_empty() {
        cli.scan_dir
    } else {
        file.scan_dirs.unwrap_or_default()
    };

    let bind = cli
        .bind
        .or(file.server.bind)
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());

    let jwt_secret = cli.jwt_secret.or(file.server.jwt_secret);
    let public_url = cli.public_url.or(file.server.public_url);

    let oidc_issuer = cli.oidc_issuer.or(file.oidc.issuer);
    let oidc_audience = cli.oidc_audience.or(file.oidc.audience);
    let oidc_client_id = cli.oidc_client_id.or(file.oidc.client_id);
    let oidc_client_secret = cli.oidc_client_secret.or(file.oidc.client_secret);
    let oidc_role_claim = cli
        .oidc_role_claim
        .or(file.oidc.role_claim)
        .unwrap_or_else(|| "groups".to_string());
    let oidc_admin_group = cli.oidc_admin_group.or(file.oidc.admin_group);

    Ok(Config {
        db,
        scan_dir,
        bind,
        jwt_secret,
        public_url,
        oidc_issuer,
        oidc_audience,
        oidc_client_id,
        oidc_client_secret,
        oidc_role_claim,
        oidc_admin_group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_cli() -> CliArgs {
        CliArgs {
            config: None,
            db: None,
            scan_dir: vec![],
            bind: None,
            jwt_secret: None,
            oidc_issuer: None,
            oidc_audience: None,
            oidc_client_id: None,
            oidc_client_secret: None,
            public_url: None,
            oidc_role_claim: None,
            oidc_admin_group: None,
        }
    }

    #[test]
    fn toml_only() {
        let file: FileConfig = toml::from_str(
            r#"
            db = "sqlite:///tmp/test.db"
            scan_dirs = ["/music"]

            [server]
            bind = "127.0.0.1:8080"
            jwt_secret = "super-secret-key-that-is-32-chars!"
            public_url = "https://music.example.com"

            [oidc]
            issuer = "https://auth.example.com"
            audience = "hummingbird"
            client_id = "hb-client"
            client_secret = "hb-secret"
            role_claim = "roles"
            admin_group = "admins"
            "#,
        )
        .unwrap();

        let config = merge(empty_cli(), file).unwrap();
        assert_eq!(config.db, "sqlite:///tmp/test.db");
        assert_eq!(config.scan_dir, vec![Utf8PathBuf::from("/music")]);
        assert_eq!(config.bind, "127.0.0.1:8080");
        assert_eq!(
            config.jwt_secret.as_deref(),
            Some("super-secret-key-that-is-32-chars!")
        );
        assert_eq!(
            config.public_url.as_deref(),
            Some("https://music.example.com")
        );
        assert_eq!(
            config.oidc_issuer.as_deref(),
            Some("https://auth.example.com")
        );
        assert_eq!(config.oidc_audience.as_deref(), Some("hummingbird"));
        assert_eq!(config.oidc_client_id.as_deref(), Some("hb-client"));
        assert_eq!(config.oidc_client_secret.as_deref(), Some("hb-secret"));
        assert_eq!(config.oidc_role_claim, "roles");
        assert_eq!(config.oidc_admin_group.as_deref(), Some("admins"));
    }

    #[test]
    fn cli_only_backward_compat() {
        let cli = CliArgs {
            db: Some("postgres://localhost/hb".into()),
            scan_dir: vec![Utf8PathBuf::from("/lib")],
            bind: Some("0.0.0.0:9000".into()),
            ..empty_cli()
        };
        let config = merge(cli, FileConfig::default()).unwrap();
        assert_eq!(config.db, "postgres://localhost/hb");
        assert_eq!(config.scan_dir, vec![Utf8PathBuf::from("/lib")]);
        assert_eq!(config.bind, "0.0.0.0:9000");
        assert_eq!(config.oidc_role_claim, "groups");
    }

    #[test]
    fn cli_overrides_toml() {
        let cli = CliArgs {
            db: Some("postgres://cli".into()),
            bind: Some("cli:1234".into()),
            oidc_role_claim: Some("cli_roles".into()),
            ..empty_cli()
        };
        let file: FileConfig = toml::from_str(
            r#"
            db = "sqlite:///toml.db"
            [server]
            bind = "toml:5678"
            [oidc]
            role_claim = "toml_roles"
            "#,
        )
        .unwrap();

        let config = merge(cli, file).unwrap();
        assert_eq!(config.db, "postgres://cli");
        assert_eq!(config.bind, "cli:1234");
        assert_eq!(config.oidc_role_claim, "cli_roles");
    }

    #[test]
    fn defaults_applied() {
        let cli = CliArgs {
            db: Some("sqlite:///test.db".into()),
            ..empty_cli()
        };
        let config = merge(cli, FileConfig::default()).unwrap();
        assert_eq!(config.bind, "0.0.0.0:3000");
        assert_eq!(config.oidc_role_claim, "groups");
        assert!(config.jwt_secret.is_none());
        assert!(config.scan_dir.is_empty());
    }

    #[test]
    fn missing_required_db_errors() {
        let result = merge(empty_cli(), FileConfig::default());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("database URL is required"));
    }

    #[test]
    fn scan_dir_cli_replaces_toml() {
        let cli = CliArgs {
            db: Some("sqlite:///test.db".into()),
            scan_dir: vec![Utf8PathBuf::from("/cli-music")],
            ..empty_cli()
        };
        let file: FileConfig = toml::from_str(
            r#"
            db = "sqlite:///test.db"
            scan_dirs = ["/toml-music1", "/toml-music2"]
            "#,
        )
        .unwrap();

        let config = merge(cli, file).unwrap();
        assert_eq!(config.scan_dir, vec![Utf8PathBuf::from("/cli-music")]);
    }

    #[test]
    fn scan_dir_from_toml_only() {
        let cli = CliArgs {
            db: Some("sqlite:///test.db".into()),
            ..empty_cli()
        };
        let file: FileConfig = toml::from_str(
            r#"
            db = "sqlite:///test.db"
            scan_dirs = ["/toml-a", "/toml-b"]
            "#,
        )
        .unwrap();

        let config = merge(cli, file).unwrap();
        assert_eq!(
            config.scan_dir,
            vec![
                Utf8PathBuf::from("/toml-a"),
                Utf8PathBuf::from("/toml-b"),
            ]
        );
    }

    #[test]
    fn partial_toml_oidc_only() {
        let cli = CliArgs {
            db: Some("sqlite:///test.db".into()),
            ..empty_cli()
        };
        let file: FileConfig = toml::from_str(
            r#"
            [oidc]
            issuer = "https://auth.example.com"
            client_id = "hb"
            "#,
        )
        .unwrap();

        let config = merge(cli, file).unwrap();
        assert_eq!(
            config.oidc_issuer.as_deref(),
            Some("https://auth.example.com")
        );
        assert_eq!(config.oidc_client_id.as_deref(), Some("hb"));
        assert!(config.oidc_audience.is_none());
        assert_eq!(config.bind, "0.0.0.0:3000");
    }

    #[test]
    fn malformed_toml_errors() {
        let result: std::result::Result<FileConfig, _> = toml::from_str("db = [not valid");
        assert!(result.is_err());
    }

    #[test]
    fn unknown_keys_rejected() {
        let result: std::result::Result<FileConfig, _> = toml::from_str(
            r#"
            db = "sqlite:///test.db"
            typo_field = "oops"
            "#,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown field"));
    }

    #[test]
    fn unknown_keys_in_nested_rejected() {
        let result: std::result::Result<FileConfig, _> = toml::from_str(
            r#"
            [server]
            typo = "oops"
            "#,
        );
        assert!(result.is_err());

        let result: std::result::Result<FileConfig, _> = toml::from_str(
            r#"
            [oidc]
            typo = "oops"
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn integration_tempfile_toml() {
        use std::io::Write;

        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
            db = "sqlite:///tmp/integration.db"
            scan_dirs = ["/music"]
            [server]
            bind = "127.0.0.1:4000"
            [oidc]
            issuer = "https://auth.test"
            "#,
        )
        .unwrap();

        let cli = CliArgs::try_parse_from([
            "hummingbird-server",
            "--config",
            tmp.path().to_str().unwrap(),
        ])
        .unwrap();

        let content = std::fs::read_to_string(cli.config.as_ref().unwrap()).unwrap();
        let file: FileConfig = toml::from_str(&content).unwrap();
        let config = merge(
            CliArgs {
                config: None,
                ..cli
            },
            file,
        )
        .unwrap();

        assert_eq!(config.db, "sqlite:///tmp/integration.db");
        assert_eq!(config.bind, "127.0.0.1:4000");
        assert_eq!(config.oidc_issuer.as_deref(), Some("https://auth.test"));
    }
}
