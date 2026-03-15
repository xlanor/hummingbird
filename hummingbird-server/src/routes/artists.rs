use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ListParams {
    sort: Option<ArtistSort>,
    order: Option<SortOrder>,
}

pub async fn list_artists(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ArtistSummary>>, AppError> {
    let sort = params.sort.unwrap_or(ArtistSort::Name);
    let order = params.order.unwrap_or_default();
    let artists = state.repo.list_artists(sort, order).await?;
    Ok(Json(artists))
}

pub async fn get_artist(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Artist>, AppError> {
    let artist = state.repo.get_artist(id).await?;
    Ok(Json(artist))
}

pub async fn get_artist_albums(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<AlbumSummary>>, AppError> {
    let albums = state.repo.get_artist_albums(id).await?;
    Ok(Json(albums))
}
