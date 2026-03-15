use std::sync::Arc;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::errors::AppError;
use crate::routes::AppState;

pub async fn get_album_art(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let art = state.repo.get_album_art(id).await?;
    match art {
        Some(blob) => {
            let mime = blob.mime.unwrap_or_else(|| "image/jpeg".to_string());
            Ok(([(header::CONTENT_TYPE, mime)], blob.data).into_response())
        }
        None => Err(AppError::NotFound),
    }
}

pub async fn get_album_thumb(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let art = state.repo.get_album_thumb(id).await?;
    match art {
        Some(blob) => {
            let mime = blob.mime.unwrap_or_else(|| "image/bmp".to_string());
            Ok(([(header::CONTENT_TYPE, mime)], blob.data).into_response())
        }
        None => Err(AppError::NotFound),
    }
}
