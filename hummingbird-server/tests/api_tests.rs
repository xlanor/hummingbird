use std::sync::Arc;

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use hummingbird_server::db::sqlite::SqliteRepository;
use hummingbird_server::db::Repository;
use hummingbird_server::models::*;
use hummingbird_server::routes::{self, AppState};
use hummingbird_server::scanner;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

async fn setup_app() -> axum::Router {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.unwrap();
    let repo: Arc<dyn Repository> = Arc::new(repo);

    let scan_handle = scanner::start_scanner(repo.clone(), vec![]);
    let state = Arc::new(AppState { repo, scan_handle });
    routes::router(state)
}

async fn setup_seeded_app() -> axum::Router {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.unwrap();

    // Seed data
    let artist_id = repo.upsert_artist("The Beatles").await.unwrap();
    let album = ScannedAlbum {
        title: "Abbey Road".into(),
        title_sortable: "abbey road".into(),
        artist_id,
        image: Some(vec![0xFF, 0xD8]),
        thumb: Some(vec![0x42, 0x4D]),
        release_date: Some("1969-09-26".into()),
        date_precision: Some(1),
        label: Some("Apple Records".into()),
        catalog_number: Some("PCS 7088".into()),
        isrc: None,
        mbid: "none".into(),
        vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();

    for i in 1..=3 {
        let track = ScannedTrack {
            title: format!("Track {i}"),
            title_sortable: format!("track {i}"),
            album_id: Some(album_id),
            track_number: Some(i),
            disc_number: Some(1),
            duration: 200000 + (i as i64 * 10000),
            location: format!("/music/beatles/track{i}.flac"),
            genres: Some("Rock".into()),
            artist_names: Some("The Beatles".into()),
            folder: Some("/music/beatles".into()),
        };
        repo.upsert_track(&track).await.unwrap();
    }

    let repo: Arc<dyn Repository> = Arc::new(repo);
    let scan_handle = scanner::start_scanner(repo.clone(), vec![]);
    let state = Arc::new(AppState { repo, scan_handle });
    routes::router(state)
}

async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn delete(app: axum::Router, uri: &str) -> StatusCode {
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::DELETE)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

// ─── Album Endpoints ───

#[tokio::test]
async fn test_api_list_albums() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/albums").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Abbey Road");
    assert_eq!(arr[0]["artist_name"], "The Beatles");
}

#[tokio::test]
async fn test_api_list_albums_empty() {
    let app = setup_app().await;
    let (status, json) = get_json(app, "/api/v1/albums").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_api_list_albums_with_sort() {
    let app = setup_seeded_app().await;
    let (status, _) = get_json(app, "/api/v1/albums?sort=title&order=desc").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_api_get_album() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/albums/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["title"], "Abbey Road");
    assert_eq!(json["label"], "Apple Records");
    assert_eq!(json["release_date"], "1969-09-26");
}

#[tokio::test]
async fn test_api_get_album_not_found() {
    let app = setup_app().await;
    let (status, json) = get_json(app, "/api/v1/albums/99999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].is_string());
}

#[tokio::test]
async fn test_api_get_album_tracks() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/albums/1/tracks").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    // Should be ordered by disc then track number
    assert_eq!(arr[0]["track_number"], 1);
    assert_eq!(arr[1]["track_number"], 2);
    assert_eq!(arr[2]["track_number"], 3);
}

// ─── Album Art Endpoints ───

#[tokio::test]
async fn test_api_album_art() {
    let app = setup_seeded_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/albums/1/art")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], &[0xFF, 0xD8]);
}

#[tokio::test]
async fn test_api_album_thumb() {
    let app = setup_seeded_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/albums/1/thumb")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], &[0x42, 0x4D]);
}

#[tokio::test]
async fn test_api_album_art_not_found() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/albums/99999/art")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ─── Artist Endpoints ───

#[tokio::test]
async fn test_api_list_artists() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/artists").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "The Beatles");
    assert!(arr[0]["album_count"].as_i64().unwrap() >= 1);
    assert!(arr[0]["track_count"].as_i64().unwrap() >= 3);
}

#[tokio::test]
async fn test_api_get_artist() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/artists/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "The Beatles");
}

#[tokio::test]
async fn test_api_get_artist_not_found() {
    let app = setup_app().await;
    let (status, _) = get_json(app, "/api/v1/artists/99999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_get_artist_albums() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/artists/1/albums").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Abbey Road");
}

// ─── Track Endpoints ───

#[tokio::test]
async fn test_api_list_tracks() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/tracks").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_api_get_track() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/tracks/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["title"], "Track 1");
    assert!(json["duration"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_api_get_track_not_found() {
    let app = setup_app().await;
    let (status, _) = get_json(app, "/api/v1/tracks/99999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─── Search Endpoint ───

#[tokio::test]
async fn test_api_search() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/search?q=beatles").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!json["artists"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_search_albums() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/search?q=abbey").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!json["albums"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_search_tracks() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/search?q=Track").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!json["tracks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_search_empty_query() {
    let app = setup_seeded_app().await;
    let (status, _) = get_json(app, "/api/v1/search?q=").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_api_search_no_results() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/search?q=xyznonexistent").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["artists"].as_array().unwrap().is_empty());
    assert!(json["albums"].as_array().unwrap().is_empty());
    assert!(json["tracks"].as_array().unwrap().is_empty());
}

// ─── Stats Endpoint ───

#[tokio::test]
async fn test_api_stats_empty() {
    let app = setup_app().await;
    let (status, json) = get_json(app, "/api/v1/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["track_count"], 0);
    assert_eq!(json["total_duration"], 0);
}

#[tokio::test]
async fn test_api_stats_with_data() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["track_count"], 3);
    assert!(json["total_duration"].as_i64().unwrap() > 0);
}

// ─── Playlist Endpoints ───

#[tokio::test]
async fn test_api_list_playlists() {
    let app = setup_seeded_app().await;
    let (status, json) = get_json(app, "/api/v1/playlists").await;
    assert_eq!(status, StatusCode::OK);
    // At least "Liked Songs" from migration
    assert!(!json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_create_playlist() {
    let app = setup_app().await;
    let (status, json) = post_json(app, "/api/v1/playlists", serde_json::json!({ "name": "My Jams" })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn test_api_get_playlist() {
    let app = setup_seeded_app().await;
    // "Liked Songs" is playlist id 1
    let (status, json) = get_json(app, "/api/v1/playlists/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Liked Songs");
    assert!(json["tracks"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_playlist_crud_flow() {
    // We need to share state across requests, so we build the app state once
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.unwrap();

    // Seed a track
    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();
    let track = ScannedTrack {
        title: "T1".into(), title_sortable: "t1".into(),
        album_id: Some(album_id), track_number: Some(1), disc_number: Some(1),
        duration: 100000, location: "/music/t1.flac".into(),
        genres: None, artist_names: None, folder: None,
    };
    let track_id = repo.upsert_track(&track).await.unwrap();

    let repo: Arc<dyn Repository> = Arc::new(repo);
    let scan_handle = scanner::start_scanner(repo.clone(), vec![]);
    let state = Arc::new(AppState { repo, scan_handle });

    // Create playlist
    let app = routes::router(state.clone());
    let (status, json) = post_json(app, "/api/v1/playlists", serde_json::json!({ "name": "CRUD Test" })).await;
    assert_eq!(status, StatusCode::OK);
    let pl_id = json["id"].as_i64().unwrap();

    // Add track
    let app = routes::router(state.clone());
    let (status, json) = post_json(
        app,
        &format!("/api/v1/playlists/{pl_id}/tracks"),
        serde_json::json!({ "track_id": track_id }),
    ).await;
    assert_eq!(status, StatusCode::OK);
    let item_id = json["item_id"].as_i64().unwrap();

    // Get playlist — should have 1 track
    let app = routes::router(state.clone());
    let (status, json) = get_json(app, &format!("/api/v1/playlists/{pl_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["tracks"].as_array().unwrap().len(), 1);

    // Remove track
    let app = routes::router(state.clone());
    let status = delete(app, &format!("/api/v1/playlists/{pl_id}/tracks/{item_id}")).await;
    assert_eq!(status, StatusCode::OK);

    // Verify empty
    let app = routes::router(state.clone());
    let (status, json) = get_json(app, &format!("/api/v1/playlists/{pl_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["tracks"].as_array().unwrap().is_empty());

    // Delete playlist
    let app = routes::router(state.clone());
    let status = delete(app, &format!("/api/v1/playlists/{pl_id}")).await;
    assert_eq!(status, StatusCode::OK);

    // Verify gone
    let app = routes::router(state.clone());
    let (status, _) = get_json(app, &format!("/api/v1/playlists/{pl_id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─── Streaming Endpoint ───

#[tokio::test]
async fn test_api_stream_track_not_found_id() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tracks/99999/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_stream_track_file_missing() {
    let app = setup_seeded_app().await;
    // Track 1 points to /music/beatles/track1.flac which doesn't exist on disk
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tracks/1/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_stream_track_with_file() {
    // Create a temp file and seed a track pointing to it
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.flac");
    std::fs::write(&file_path, b"fake audio data for testing streaming").unwrap();

    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.unwrap();

    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();
    let track = ScannedTrack {
        title: "Stream Test".into(), title_sortable: "stream test".into(),
        album_id: Some(album_id), track_number: Some(1), disc_number: Some(1),
        duration: 100000, location: file_path.to_string_lossy().to_string(),
        genres: None, artist_names: None, folder: None,
    };
    repo.upsert_track(&track).await.unwrap();

    let repo: Arc<dyn Repository> = Arc::new(repo);
    let scan_handle = scanner::start_scanner(repo.clone(), vec![]);
    let state = Arc::new(AppState { repo, scan_handle });
    let app = routes::router(state);

    // Full request
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tracks/1/stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("accept-ranges").unwrap(),
        "bytes"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"fake audio data for testing streaming");
}

#[tokio::test]
async fn test_api_stream_track_range_request() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.flac");
    std::fs::write(&file_path, b"0123456789ABCDEF").unwrap();

    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();

    let repo = SqliteRepository::new(pool);
    repo.run_migrations().await.unwrap();

    let artist_id = repo.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = repo.upsert_album(&album).await.unwrap();
    let track = ScannedTrack {
        title: "Range Test".into(), title_sortable: "range test".into(),
        album_id: Some(album_id), track_number: Some(1), disc_number: Some(1),
        duration: 100000, location: file_path.to_string_lossy().to_string(),
        genres: None, artist_names: None, folder: None,
    };
    repo.upsert_track(&track).await.unwrap();

    let repo: Arc<dyn Repository> = Arc::new(repo);
    let scan_handle = scanner::start_scanner(repo.clone(), vec![]);
    let state = Arc::new(AppState { repo, scan_handle });
    let app = routes::router(state);

    // Range request: bytes 4-7
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tracks/1/stream")
                .header("range", "bytes=4-7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        resp.headers().get("content-range").unwrap(),
        "bytes 4-7/16"
    );
    assert_eq!(
        resp.headers().get("content-length").unwrap(),
        "4"
    );
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"4567");
}

// ─── Scanner Endpoints ───

#[tokio::test]
async fn test_api_trigger_scan() {
    let app = setup_app().await;
    let (status, json) = post_json(app, "/api/v1/scan", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "scan_started");
}

#[tokio::test]
async fn test_api_force_scan() {
    let app = setup_app().await;
    let (status, json) = post_json(app, "/api/v1/scan/force", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "force_scan_started");
}

// ─── 404 for unknown routes ───

#[tokio::test]
async fn test_api_unknown_route() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

