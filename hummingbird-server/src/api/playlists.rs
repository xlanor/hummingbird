use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::domain::playlist::*;
use crate::errors::AppError;
use crate::infrastructure::auth::AuthUser;
use crate::api::AppState;

pub async fn list_playlists(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<Vec<PlaylistWithCount>>, AppError> {
    let playlists = state.db.list_playlists(auth_user.user_id).await?;
    Ok(Json(playlists))
}

pub async fn get_playlist(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<PlaylistDetail>, AppError> {
    verify_playlist_access(&state, &auth_user, id).await?;
    let detail = state.db.get_playlist(id).await?;
    Ok(Json(detail))
}

#[derive(Deserialize)]
pub struct CreatePlaylist {
    name: String,
}

pub async fn create_playlist(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(body): Json<CreatePlaylist>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = state.db.create_playlist(&body.name, auth_user.user_id).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn delete_playlist(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_playlist_access(&state, &auth_user, id).await?;
    state.db.delete_playlist(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AddTrack {
    track_id: i64,
}

pub async fn add_track(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<AddTrack>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_playlist_access(&state, &auth_user, id).await?;
    let item_id = state.db.add_to_playlist(id, body.track_id).await?;
    Ok(Json(serde_json::json!({ "item_id": item_id })))
}

pub async fn remove_track(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path((id, item_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_playlist_access(&state, &auth_user, id).await?;
    state.db.remove_from_playlist(item_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct MoveTrack {
    position: i32,
}

pub async fn move_track(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Path((id, item_id)): Path<(i64, i64)>,
    Json(body): Json<MoveTrack>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_playlist_access(&state, &auth_user, id).await?;
    state.db.move_playlist_item(item_id, body.position).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn verify_playlist_access(
    state: &Arc<AppState>,
    auth_user: &AuthUser,
    playlist_id: i64,
) -> Result<(), AppError> {
    if auth_user.is_admin() {
        return Ok(());
    }
    let owner_id = state.db.get_playlist_owner(playlist_id).await?;
    if owner_id != auth_user.user_id {
        return Err(AppError::Forbidden);
    }
    Ok(())
}
