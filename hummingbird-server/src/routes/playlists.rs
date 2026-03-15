use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::routes::AppState;

pub async fn list_playlists(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PlaylistWithCount>>, AppError> {
    let playlists = state.repo.list_playlists().await?;
    Ok(Json(playlists))
}

pub async fn get_playlist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PlaylistDetail>, AppError> {
    let detail = state.repo.get_playlist(id).await?;
    Ok(Json(detail))
}

#[derive(Deserialize)]
pub struct CreatePlaylist {
    name: String,
}

pub async fn create_playlist(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePlaylist>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = state.repo.create_playlist(&body.name).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn delete_playlist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.repo.delete_playlist(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct AddTrack {
    track_id: i64,
}

pub async fn add_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<AddTrack>,
) -> Result<Json<serde_json::Value>, AppError> {
    let item_id = state.repo.add_to_playlist(id, body.track_id).await?;
    Ok(Json(serde_json::json!({ "item_id": item_id })))
}

pub async fn remove_track(
    State(state): State<Arc<AppState>>,
    Path((_, item_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.repo.remove_from_playlist(item_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct MoveTrack {
    position: i32,
}

pub async fn move_track(
    State(state): State<Arc<AppState>>,
    Path((_, item_id)): Path<(i64, i64)>,
    Json(body): Json<MoveTrack>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.repo.move_playlist_item(item_id, body.position).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
