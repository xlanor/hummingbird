use std::sync::Arc;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::errors::AppError;
use crate::models::SearchResults;
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResults>, AppError> {
    if params.q.is_empty() {
        return Err(AppError::BadRequest("query parameter 'q' is required".into()));
    }
    let results = state.repo.search(&params.q).await?;
    Ok(Json(results))
}
