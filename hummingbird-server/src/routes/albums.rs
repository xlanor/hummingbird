use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ListParams {
    sort: Option<AlbumSort>,
    order: Option<SortOrder>,
}

pub async fn list_albums(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<AlbumSummary>>, AppError> {
    let sort = params.sort.unwrap_or(AlbumSort::Title);
    let order = params.order.unwrap_or_default();
    let albums = state.repo.list_albums(sort, order).await?;
    Ok(Json(albums))
}

pub async fn get_album(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Album>, AppError> {
    let album = state.repo.get_album(id).await?;
    Ok(Json(album))
}

pub async fn get_album_tracks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Track>>, AppError> {
    let tracks = state.repo.get_album_tracks(id).await?;
    Ok(Json(tracks))
}
