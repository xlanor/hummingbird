use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::domain::user::User;
use crate::errors::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    #[serde(default)]
    pub token_type: Option<String>,
}

pub fn issue_token(user: &User, secret: &[u8], ttl_hours: u64) -> Result<String, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now,
        exp: now + (ttl_hours as usize * 3600),
        iss: "local".to_string(),
        token_type: Some("access".to_string()),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to sign token: {e}")))
}

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

pub fn issue_token_pair(user: &User, secret: &[u8]) -> Result<TokenPair, AppError> {
    let now = chrono::Utc::now().timestamp() as usize;

    let access_exp = 15 * 60; // 15 minutes
    let access_claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now,
        exp: now + access_exp,
        iss: "local".to_string(),
        token_type: Some("access".to_string()),
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to sign access token: {e}")))?;

    let refresh_exp = 7 * 24 * 3600; // 7 days
    let refresh_claims = Claims {
        sub: user.id.to_string(),
        username: String::new(),
        role: String::new(),
        iat: now,
        exp: now + refresh_exp,
        iss: "local".to_string(),
        token_type: Some("refresh".to_string()),
    };

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to sign refresh token: {e}")))?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_in: access_exp as u64,
    })
}

pub fn validate_refresh_token(token: &str, secret: &[u8]) -> Result<i64, AppError> {
    let mut validation = Validation::default();
    validation.set_issuer(&["local"]);

    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| AppError::Unauthorized(format!("invalid refresh token: {e}")))?;

    match data.claims.token_type.as_deref() {
        Some("refresh") => {}
        _ => return Err(AppError::Unauthorized("not a refresh token".into())),
    }

    let user_id: i64 = data
        .claims
        .sub
        .parse()
        .map_err(|_| AppError::Unauthorized("invalid sub in refresh token".into()))?;

    Ok(user_id)
}
