use std::sync::Arc;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::errors::AppError;
use crate::api::AppState;

pub async fn stream_track(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let track = state.db.get_track(id).await?;
    let path = std::path::Path::new(&track.location);

    if !path.exists() {
        return Err(AppError::NotFound);
    }

    let metadata = tokio::fs::metadata(path).await?;
    let total_size = metadata.len();

    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    if let Some(range) = headers.get(header::RANGE) {
        let range_str = range.to_str().unwrap_or("");
        if let Some((start, end)) = parse_range(range_str, total_size) {
            let content_length = end - start + 1;

            let mut file = tokio::fs::File::open(path).await?;
            file.seek(std::io::SeekFrom::Start(start)).await?;

            let reader = file.take(content_length);
            let stream = tokio_util::io::ReaderStream::new(reader);
            let body = Body::from_stream(stream);

            return Ok((
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CONTENT_LENGTH, content_length.to_string()),
                    (
                        header::CONTENT_RANGE,
                        format!("bytes {start}-{end}/{total_size}"),
                    ),
                    (header::ACCEPT_RANGES, "bytes".to_string()),
                ],
                body,
            )
                .into_response());
        }
    }

    let file = tokio::fs::File::open(path).await?;
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CONTENT_LENGTH, total_size.to_string()),
            (header::ACCEPT_RANGES, "bytes".to_string()),
        ],
        body,
    )
        .into_response())
}

pub(crate) fn parse_range(range: &str, total: u64) -> Option<(u64, u64)> {
    let range = range.strip_prefix("bytes=")?;
    let mut parts = range.split('-');
    let start_str = parts.next()?.trim();
    let end_str = parts.next()?.trim();

    if start_str.is_empty() {
        let suffix: u64 = end_str.parse().ok()?;
        let start = total.saturating_sub(suffix);
        Some((start, total - 1))
    } else {
        let start: u64 = start_str.parse().ok()?;
        let end = if end_str.is_empty() {
            total - 1
        } else {
            end_str.parse().ok()?
        };
        if start > end || start >= total {
            return None;
        }
        Some((start, end.min(total - 1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE_SIZE: u64 = 10000;

    #[test]
    fn test_parse_range_full_range() {
        assert_eq!(parse_range("bytes=0-999", FILE_SIZE), Some((0, 999)));
    }

    #[test]
    fn test_parse_range_open_end() {
        assert_eq!(parse_range("bytes=500-", FILE_SIZE), Some((500, 9999)));
    }

    #[test]
    fn test_parse_range_suffix() {
        assert_eq!(parse_range("bytes=-500", FILE_SIZE), Some((9500, 9999)));
    }

    #[test]
    fn test_parse_range_first_byte() {
        assert_eq!(parse_range("bytes=0-0", FILE_SIZE), Some((0, 0)));
    }

    #[test]
    fn test_parse_range_last_byte() {
        assert_eq!(parse_range("bytes=9999-9999", FILE_SIZE), Some((9999, 9999)));
    }

    #[test]
    fn test_parse_range_clamps_end() {
        assert_eq!(parse_range("bytes=0-99999", FILE_SIZE), Some((0, 9999)));
    }

    #[test]
    fn test_parse_range_start_beyond_file() {
        assert_eq!(parse_range("bytes=10000-", FILE_SIZE), None);
    }

    #[test]
    fn test_parse_range_start_after_end() {
        assert_eq!(parse_range("bytes=500-100", FILE_SIZE), None);
    }

    #[test]
    fn test_parse_range_no_prefix() {
        assert_eq!(parse_range("0-100", FILE_SIZE), None);
    }

    #[test]
    fn test_parse_range_suffix_larger_than_file() {
        assert_eq!(parse_range("bytes=-20000", FILE_SIZE), Some((0, 9999)));
    }
}
