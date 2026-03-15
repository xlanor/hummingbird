use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use axum::response::sse::{Event, Sse};
use tokio_stream::StreamExt;

use crate::domain::scanner::ScanStatus;
use crate::errors::AppError;
use crate::api::AppState;

pub async fn trigger_scan(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.scan_handle.trigger_scan(false);
    Ok(Json(serde_json::json!({ "status": "scan_started" })))
}

pub async fn force_scan(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.scan_handle.trigger_scan(true);
    Ok(Json(serde_json::json!({ "status": "force_scan_started" })))
}

pub async fn scan_status(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.scan_handle.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|result: Result<ScanStatus, _>| result.ok())
        .map(|status: ScanStatus| {
            let data = match &status {
                ScanStatus::Idle => serde_json::json!({ "status": "idle" }),
                ScanStatus::Scanning { processed, total } => {
                    serde_json::json!({ "status": "scanning", "processed": processed, "total": total })
                }
                ScanStatus::Complete { tracks_found } => {
                    serde_json::json!({ "status": "complete", "tracks_found": tracks_found })
                }
            };
            Ok(Event::default().data(data.to_string()))
        });

    Sse::new(stream)
}
