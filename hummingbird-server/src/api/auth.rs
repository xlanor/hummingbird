use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use ring::digest;
use serde::{Deserialize, Serialize};

use crate::api::AppState;
use crate::domain::user::UserPublic;
use crate::errors::AppError;
use crate::infrastructure::auth::{self, AuthUser};

// ---------------------------------------------------------------------------
// Login (password-based)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: UserPublic,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    if state.oidc_only {
        return Err(AppError::BadRequest(
            "password login is disabled when OIDC is configured".into(),
        ));
    }

    let user = state
        .db
        .get_user_by_username(&body.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid credentials".into()))?;

    let hash = user
        .password_hash
        .as_deref()
        .ok_or_else(|| AppError::Unauthorized("this account uses OIDC login".into()))?;

    if !auth::verify_password(&body.password, hash)? {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    let pair = auth::issue_token_pair(&user, &state.jwt_secret)?;

    Ok(Json(TokenResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
        user: user.into(),
    }))
}

// ---------------------------------------------------------------------------
// GET /auth/providers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc: Option<OidcProviderInfo>,
}

#[derive(Serialize)]
pub struct OidcProviderInfo {
    pub enabled: bool,
    pub authorize_url: String,
}

pub async fn providers(
    State(state): State<Arc<AppState>>,
) -> Json<ProvidersResponse> {
    let oidc = if state.oidc_only {
        let public_url = state.public_url.as_deref().unwrap_or("");
        Some(OidcProviderInfo {
            enabled: true,
            authorize_url: format!("{public_url}/api/v1/auth/oidc/authorize"),
        })
    } else {
        None
    };

    Json(ProvidersResponse {
        password: !state.oidc_only,
        oidc,
    })
}

// ---------------------------------------------------------------------------
// GET /auth/oidc/authorize
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct AuthorizeQuery {
    redirect_uri: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PkceState {
    code_verifier: String,
    redirect_uri: String,
    exp: usize,
}

pub async fn oidc_authorize(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuthorizeQuery>,
) -> Result<Redirect, AppError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".into()))?;

    if !oidc.auth_code_enabled() {
        return Err(AppError::BadRequest("OIDC auth code flow is not enabled".into()));
    }

    let authorization_endpoint = oidc
        .authorization_endpoint
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("no authorization_endpoint")))?;

    let public_url = state
        .public_url
        .as_deref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("public_url not configured")))?;

    let callback_url = format!("{public_url}/api/v1/auth/oidc/callback");

    let frontend_redirect = query
        .redirect_uri
        .unwrap_or_else(|| format!("{public_url}/"));

    // Generate PKCE
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Encode state as JWT
    let now = chrono::Utc::now().timestamp() as usize;
    let pkce_state = PkceState {
        code_verifier,
        redirect_uri: frontend_redirect,
        exp: now + 600, // 10 minutes
    };
    let state_jwt = encode(
        &Header::default(),
        &pkce_state,
        &EncodingKey::from_secret(&state.jwt_secret),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to encode state: {e}")))?;

    let client_id = oidc.client_id.as_deref().unwrap();

    let auth_url = format!(
        "{authorization_endpoint}?response_type=code&client_id={client_id}&redirect_uri={callback}&scope=openid%20profile%20email&code_challenge={challenge}&code_challenge_method=S256&state={state}",
        callback = urlencoding::encode(&callback_url),
        challenge = urlencoding::encode(&code_challenge),
        state = urlencoding::encode(&state_jwt),
    );

    Ok(Redirect::temporary(&auth_url))
}

// ---------------------------------------------------------------------------
// GET /auth/oidc/callback
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

pub async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> Result<Redirect, AppError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC is not configured".into()))?;

    // Decode state JWT to get code_verifier and redirect_uri
    let mut validation = Validation::default();
    validation.insecure_disable_signature_validation();
    validation.set_required_spec_claims::<&str>(&[]);
    // Re-validate with proper key
    let validation2 = Validation::default();
    let state_data = decode::<PkceState>(
        &query.state,
        &DecodingKey::from_secret(&state.jwt_secret),
        &validation2,
    )
    .map_err(|e| AppError::BadRequest(format!("invalid state parameter: {e}")))?;

    let pkce = state_data.claims;
    let public_url = state.public_url.as_deref().unwrap_or("");
    let callback_url = format!("{public_url}/api/v1/auth/oidc/callback");

    // Exchange code for tokens
    let token_resp = auth::exchange_code(oidc, &query.code, &pkce.code_verifier, &callback_url)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("token exchange failed: {e}")))?;

    // Parse the id_token (or fall back to access_token) to get claims
    let raw_token = token_resp.id_token.as_deref().unwrap_or(&token_resp.access_token);
    let id_claims = decode_id_token_unverified(raw_token)?;

    let subject = id_claims["sub"]
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("missing sub in id_token")))?;

    let username = id_claims["preferred_username"]
        .as_str()
        .or_else(|| id_claims["email"].as_str())
        .unwrap_or(subject);

    let display_name = id_claims["name"].as_str();

    // Create or get user
    let user = state
        .db
        .create_or_get_oidc_user(&oidc.issuer, subject, username, display_name)
        .await?;

    // Map role from claims
    let role = auth::extract_role(&id_claims, &oidc.role_claim, oidc.admin_group.as_deref());
    if role != user.role {
        state.db.update_user_role(user.id, &role).await?;
    }

    // Re-fetch user with updated role
    let user = state.db.get_user_by_id(user.id).await?;

    // Issue local token pair
    let pair = auth::issue_token_pair(&user, &state.jwt_secret)?;

    // Redirect to frontend with tokens in fragment
    let redirect_url = format!(
        "{}#access_token={}&refresh_token={}&expires_in={}",
        pkce.redirect_uri,
        urlencoding::encode(&pair.access_token),
        urlencoding::encode(&pair.refresh_token),
        pair.expires_in,
    );

    Ok(Redirect::temporary(&redirect_url))
}

fn decode_id_token_unverified(token: &str) -> Result<serde_json::Value, AppError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::Internal(anyhow::anyhow!("invalid JWT format")));
    }
    let payload = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid base64 in id_token: {e}")))?;
    let claims: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid JSON in id_token: {e}")))?;
    Ok(claims)
}

// ---------------------------------------------------------------------------
// POST /auth/refresh
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    let user_id = auth::validate_refresh_token(&body.refresh_token, &state.jwt_secret)?;
    let user = state
        .db
        .get_user_by_id(user_id)
        .await
        .map_err(|_| AppError::Unauthorized("user no longer exists".into()))?;

    let pair = auth::issue_token_pair(&user, &state.jwt_secret)?;

    Ok(Json(RefreshResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        expires_in: pair.expires_in,
    }))
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

fn generate_code_verifier() -> String {
    use rand::Rng;
    let mut buf = [0u8; 32];
    rand::rng().fill(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn generate_code_challenge(verifier: &str) -> String {
    let hash = digest::digest(&digest::SHA256, verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash.as_ref())
}

// ---------------------------------------------------------------------------
// Existing endpoints (unchanged)
// ---------------------------------------------------------------------------

pub async fn me(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<UserPublic>, AppError> {
    let user = state.db.get_user_by_id(auth_user.user_id).await?;
    Ok(Json(user.into()))
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    username: String,
    password: String,
    display_name: Option<String>,
    #[serde(default = "default_role")]
    role: String,
}

fn default_role() -> String {
    "user".into()
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(body): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden);
    }

    if body.role != "user" && body.role != "admin" {
        return Err(AppError::BadRequest("role must be 'user' or 'admin'".into()));
    }

    if body.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".into()));
    }

    let hash = auth::hash_password(&body.password)?;
    let id = state
        .db
        .create_user(&body.username, body.display_name.as_deref(), Some(&hash), &body.role)
        .await?;

    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<UserPublic>>, AppError> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden);
    }

    let users = state.db.list_users().await?;
    Ok(Json(users.into_iter().map(UserPublic::from).collect()))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !auth_user.is_admin() {
        return Err(AppError::Forbidden);
    }

    if auth_user.user_id == id {
        return Err(AppError::BadRequest("cannot delete yourself".into()));
    }

    state.db.delete_user(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    password: String,
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".into()));
    }

    let hash = auth::hash_password(&body.password)?;
    state
        .db
        .update_user_password(auth_user.user_id, &hash)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
