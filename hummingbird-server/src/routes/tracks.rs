use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::*;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct ListParams {
    sort: Option<TrackSort>,
    order: Option<SortOrder>,
}

pub async fn list_tracks(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Track>>, AppError> {
    let sort = params.sort.unwrap_or(TrackSort::Title);
    let order = params.order.unwrap_or_default();
    let tracks = state.repo.list_tracks(sort, order).await?;
    Ok(Json(tracks))
}

pub async fn get_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Track>, AppError> {
    let track = state.repo.get_track(id).await?;
    Ok(Json(track))
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LibraryStats>, AppError> {
    let stats = state.repo.get_stats().await?;
    Ok(Json(stats))
}
