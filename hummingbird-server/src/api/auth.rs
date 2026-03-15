use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::domain::user::UserPublic;
use crate::errors::AppError;
use crate::infrastructure::auth::{self, AuthUser};
use crate::api::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Serialize)]
pub struct TokenResponse {
    token: String,
    user: UserPublic,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, AppError> {
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

    let token = auth::issue_token(&user, &state.jwt_secret, 24)?;

    Ok(Json(TokenResponse {
        token,
        user: user.into(),
    }))
}

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
