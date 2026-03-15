use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks: Arc<RwLock<jsonwebtoken::jwk::JwkSet>>,
    pub authorization_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub role_claim: String,
    pub admin_group: Option<String>,
}

impl OidcConfig {
    pub fn auth_code_enabled(&self) -> bool {
        self.client_id.is_some()
    }
}

pub struct DiscoverParams<'a> {
    pub issuer: &'a str,
    pub audience: &'a str,
    pub client_id: Option<&'a str>,
    pub client_secret: Option<&'a str>,
    pub role_claim: &'a str,
    pub admin_group: Option<&'a str>,
}

pub async fn discover_oidc(params: DiscoverParams<'_>) -> anyhow::Result<OidcConfig> {
    let issuer_trimmed = params.issuer.trim_end_matches('/');
    let discovery_url = format!("{issuer_trimmed}/.well-known/openid-configuration");

    let client = reqwest::Client::new();
    let discovery: serde_json::Value = client
        .get(&discovery_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let jwks_uri = discovery["jwks_uri"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing jwks_uri in OIDC discovery"))?;

    let jwks: jsonwebtoken::jwk::JwkSet =
        client.get(jwks_uri).send().await?.error_for_status()?.json().await?;

    let authorization_endpoint = discovery["authorization_endpoint"].as_str().map(String::from);
    let token_endpoint = discovery["token_endpoint"].as_str().map(String::from);

    Ok(OidcConfig {
        issuer: issuer_trimmed.to_string(),
        audience: params.audience.to_string(),
        jwks: Arc::new(RwLock::new(jwks)),
        authorization_endpoint,
        token_endpoint,
        client_id: params.client_id.map(String::from),
        client_secret: params.client_secret.map(String::from),
        role_claim: params.role_claim.to_string(),
        admin_group: params.admin_group.map(String::from),
    })
}

#[derive(Debug, Deserialize)]
pub struct TokenExchangeResponse {
    pub id_token: Option<String>,
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
}

pub async fn exchange_code(
    oidc: &OidcConfig,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> anyhow::Result<TokenExchangeResponse> {
    let token_endpoint = oidc
        .token_endpoint
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("OIDC token_endpoint not available"))?;

    let client_id = oidc
        .client_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("OIDC client_id not configured"))?;

    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", client_id),
        ("code_verifier", code_verifier),
    ];

    let secret_val;
    if let Some(ref secret) = oidc.client_secret {
        secret_val = secret.clone();
        params.push(("client_secret", &secret_val));
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(token_endpoint)
        .form(&params)
        .send()
        .await?
        .error_for_status()?;

    let token_resp: TokenExchangeResponse = resp.json().await?;
    Ok(token_resp)
}

pub fn extract_role(claims: &serde_json::Value, role_claim: &str, admin_group: Option<&str>) -> String {
    let admin_group = match admin_group {
        Some(g) => g,
        None => return "user".to_string(),
    };

    let value = resolve_claim(claims, role_claim);

    let is_admin = match value {
        Some(serde_json::Value::Array(arr)) => {
            arr.iter().any(|v| v.as_str() == Some(admin_group))
        }
        Some(serde_json::Value::String(s)) => s == admin_group,
        _ => false,
    };

    if is_admin { "admin".to_string() } else { "user".to_string() }
}

fn resolve_claim<'a>(claims: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = claims;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}
