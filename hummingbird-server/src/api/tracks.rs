use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::domain::library::*;
use crate::errors::AppError;
use crate::api::AppState;

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
    let tracks = state.db.list_tracks(sort, order).await?;
    Ok(Json(tracks))
}

pub async fn get_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Track>, AppError> {
    let track = state.db.get_track(id).await?;
    Ok(Json(track))
}

pub async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LibraryStats>, AppError> {
    let stats = state.db.get_stats().await?;
    Ok(Json(stats))
}
