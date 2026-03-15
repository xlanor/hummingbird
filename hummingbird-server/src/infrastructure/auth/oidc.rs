use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks: Arc<RwLock<jsonwebtoken::jwk::JwkSet>>,
}

pub async fn discover_oidc(issuer: &str, audience: &str) -> anyhow::Result<OidcConfig> {
    let issuer_trimmed = issuer.trim_end_matches('/');
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

    Ok(OidcConfig {
        issuer: issuer_trimmed.to_string(),
        audience: audience.to_string(),
        jwks: Arc::new(RwLock::new(jwks)),
    })
}
