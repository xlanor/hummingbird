use std::sync::Arc;

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, DecodingKey, Validation};
use tracing::warn;

use super::jwt::Claims;
use super::oidc::OidcConfig;
use crate::api::AppState;
use crate::errors::AppError;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("not authenticated".into()))
    }
}

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;

    if let Some(user) = try_local_jwt(&token, &state).await? {
        request.extensions_mut().insert(user);
        return Ok(next.run(request).await);
    }

    if let Some(ref oidc) = state.oidc {
        if let Some(user) = try_oidc_jwt(&token, oidc, &state).await? {
            request.extensions_mut().insert(user);
            return Ok(next.run(request).await);
        }
    }

    Err(AppError::Unauthorized("invalid token".into()))
}

async fn try_local_jwt(
    token: &str,
    state: &Arc<AppState>,
) -> Result<Option<AuthUser>, AppError> {
    let mut validation = Validation::default();
    validation.set_issuer(&["local"]);

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(&state.jwt_secret),
        &validation,
    ) {
        Ok(data) => {
            let user_id: i64 = data
                .claims
                .sub
                .parse()
                .map_err(|_| AppError::Unauthorized("invalid sub claim".into()))?;

            state.db.get_user_by_id(user_id).await.map_err(|_| {
                AppError::Unauthorized("user no longer exists".into())
            })?;

            Ok(Some(AuthUser {
                user_id,
                username: data.claims.username,
                role: data.claims.role,
            }))
        }
        Err(_) => Ok(None),
    }
}

async fn try_oidc_jwt(
    token: &str,
    oidc: &OidcConfig,
    state: &Arc<AppState>,
) -> Result<Option<AuthUser>, AppError> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|_| AppError::Unauthorized("invalid token header".into()))?;

    let kid = match header.kid {
        Some(ref k) => k.clone(),
        None => return Ok(None),
    };

    let jwks = oidc.jwks.read().await;
    let jwk = match jwks.find(&kid) {
        Some(k) => k,
        None => {
            warn!("OIDC token kid '{kid}' not found in JWKS");
            return Ok(None);
        }
    };

    let decoding_key = DecodingKey::from_jwk(jwk)
        .map_err(|e| AppError::Unauthorized(format!("invalid JWKS key: {e}")))?;

    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[&oidc.issuer]);
    validation.set_audience(&[&oidc.audience]);

    let token_data = match decode::<serde_json::Value>(token, &decoding_key, &validation) {
        Ok(d) => d,
        Err(e) => {
            warn!("OIDC token validation failed: {e}");
            return Ok(None);
        }
    };

    let subject = token_data.claims["sub"]
        .as_str()
        .ok_or_else(|| AppError::Unauthorized("missing sub in OIDC token".into()))?;

    let username = token_data.claims["preferred_username"]
        .as_str()
        .or_else(|| token_data.claims["email"].as_str())
        .unwrap_or(subject);

    let display_name = token_data.claims["name"].as_str();

    let user = state
        .db
        .create_or_get_oidc_user(&oidc.issuer, subject, username, display_name)
        .await?;

    Ok(Some(AuthUser {
        user_id: user.id,
        username: user.username,
        role: user.role,
    }))
}
