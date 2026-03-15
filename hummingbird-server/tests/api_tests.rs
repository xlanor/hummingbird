use std::sync::Arc;

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use hummingbird_server::api::{self, AppState};
use hummingbird_server::domain::scanner::dao::ScannerDao;
use hummingbird_server::domain::scanner::orchestrator;
use hummingbird_server::domain::scanner::{ScannedAlbum, ScannedTrack};
use hummingbird_server::domain::user::dao::UserDao;
use hummingbird_server::infrastructure::auth;
use hummingbird_server::infrastructure::persistence::sqlite::SqliteDatabase;
use hummingbird_server::infrastructure::persistence::Database;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

const JWT_SECRET: &[u8] = b"test-secret-key-for-unit-tests-only-32chars!!";

async fn setup_app() -> (axum::Router, String) {
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

    let db = SqliteDatabase::new(pool);
    db.run_migrations().await.unwrap();

    let hash = auth::hash_password("testpass").unwrap();
    db.create_user("testadmin", None, Some(&hash), "admin").await.unwrap();

    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);
    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: None,
    });

    let app = api::router(state.clone());
    let token = login_token(app.clone(), "testadmin", "testpass").await;
    (app, token)
}

async fn setup_seeded_app() -> (axum::Router, String, Arc<AppState>) {
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

    let db = SqliteDatabase::new(pool);
    db.run_migrations().await.unwrap();

    let hash = auth::hash_password("testpass").unwrap();
    db.create_user("testadmin", None, Some(&hash), "admin").await.unwrap();

    let artist_id = db.upsert_artist("The Beatles").await.unwrap();
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
    let album_id = db.upsert_album(&album).await.unwrap();

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
        db.upsert_track(&track).await.unwrap();
    }

    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);
    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: None,
    });

    let app = api::router(state.clone());
    let token = login_token(app.clone(), "testadmin", "testpass").await;
    (app, token, state)
}

async fn login_token(app: axum::Router, username: &str, password: &str) -> String {
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "username": username,
                        "password": password,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["token"].as_str().unwrap().to_string()
}

async fn authed_get(app: axum::Router, uri: &str, token: &str) -> (StatusCode, serde_json::Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
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

async fn authed_post(
    app: axum::Router,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri(uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
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

async fn authed_delete(app: axum::Router, uri: &str, token: &str) -> StatusCode {
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::DELETE)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status()
}

#[tokio::test]
async fn test_unauthenticated_request_returns_401() {
    let (app, _) = setup_app().await;
    let resp = app
        .oneshot(Request::builder().uri("/api/v1/albums").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_invalid_token_returns_401() {
    let (app, _) = setup_app().await;
    let (status, _) = authed_get(app, "/api/v1/albums", "garbage-token").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_success() {
    let (app, token) = setup_app().await;
    assert!(!token.is_empty());
    let (status, json) = authed_get(app, "/api/v1/auth/me", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["username"], "testadmin");
}

#[tokio::test]
async fn test_login_bad_password() {
    let (app, _) = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({"username": "testadmin", "password": "wrong"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let (app, _) = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({"username": "nobody", "password": "x"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_api_list_albums() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/albums", &token).await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["title"], "Abbey Road");
}

#[tokio::test]
async fn test_api_get_album() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/albums/1", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["title"], "Abbey Road");
}

#[tokio::test]
async fn test_api_get_album_tracks() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/albums/1/tracks", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_api_album_art() {
    let (app, token, _) = setup_seeded_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/albums/1/art")
                .header("authorization", format!("Bearer {token}"))
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
async fn test_api_list_artists() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/artists", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_api_get_artist() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/artists/1", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "The Beatles");
}

#[tokio::test]
async fn test_api_list_tracks() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/tracks", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_api_search() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/search?q=beatles", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!json["artists"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_api_stats() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_get(app, "/api/v1/stats", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["track_count"], 3);
}

#[tokio::test]
async fn test_api_playlist_crud() {
    let (_, token, state) = setup_seeded_app().await;

    let app = api::router(state.clone());
    let (status, json) = authed_post(app, "/api/v1/playlists", &token, serde_json::json!({"name": "My PL"})).await;
    assert_eq!(status, StatusCode::OK);
    let pl_id = json["id"].as_i64().unwrap();

    let app = api::router(state.clone());
    let (status, json) = authed_get(app, &format!("/api/v1/playlists/{pl_id}"), &token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "My PL");

    let app = api::router(state.clone());
    let (status, json) = authed_post(
        app,
        &format!("/api/v1/playlists/{pl_id}/tracks"),
        &token,
        serde_json::json!({"track_id": 1}),
    ).await;
    assert_eq!(status, StatusCode::OK);
    let item_id = json["item_id"].as_i64().unwrap();

    let app = api::router(state.clone());
    let status = authed_delete(app, &format!("/api/v1/playlists/{pl_id}/tracks/{item_id}"), &token).await;
    assert_eq!(status, StatusCode::OK);

    let app = api::router(state.clone());
    let status = authed_delete(app, &format!("/api/v1/playlists/{pl_id}"), &token).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_api_user_management() {
    let (_, token, state) = setup_seeded_app().await;

    let app = api::router(state.clone());
    let (status, json) = authed_post(
        app,
        "/api/v1/users",
        &token,
        serde_json::json!({"username": "newuser", "password": "password123", "role": "user"}),
    ).await;
    assert_eq!(status, StatusCode::OK);
    let user_id = json["id"].as_i64().unwrap();

    let app = api::router(state.clone());
    let (status, json) = authed_get(app, "/api/v1/users", &token).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().unwrap().len() >= 2);

    let app = api::router(state.clone());
    let status = authed_delete(app, &format!("/api/v1/users/{user_id}"), &token).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_api_non_admin_cannot_create_users() {
    let (_, admin_token, state) = setup_seeded_app().await;

    let app = api::router(state.clone());
    authed_post(
        app,
        "/api/v1/users",
        &admin_token,
        serde_json::json!({"username": "regular", "password": "password123"}),
    ).await;

    let app = api::router(state.clone());
    let user_token = login_token(app, "regular", "password123").await;

    let app = api::router(state.clone());
    let (status, _) = authed_post(
        app,
        "/api/v1/users",
        &user_token,
        serde_json::json!({"username": "hacker", "password": "password123"}),
    ).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_api_trigger_scan() {
    let (app, token, _) = setup_seeded_app().await;
    let (status, json) = authed_post(app, "/api/v1/scan", &token, serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "scan_started");
}

#[tokio::test]
async fn test_api_stream_track_with_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.flac");
    std::fs::write(&file_path, b"fake audio data").unwrap();

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

    let db = SqliteDatabase::new(pool);
    db.run_migrations().await.unwrap();

    let hash = auth::hash_password("pass").unwrap();
    db.create_user("u", None, Some(&hash), "admin").await.unwrap();

    let artist_id = db.upsert_artist("A").await.unwrap();
    let album = ScannedAlbum {
        title: "Al".into(), title_sortable: "al".into(), artist_id,
        image: None, thumb: None, release_date: None, date_precision: None,
        label: None, catalog_number: None, isrc: None,
        mbid: "none".into(), vinyl_numbering: false,
    };
    let album_id = db.upsert_album(&album).await.unwrap();
    let track = ScannedTrack {
        title: "S".into(), title_sortable: "s".into(),
        album_id: Some(album_id), track_number: Some(1), disc_number: Some(1),
        duration: 100000, location: file_path.to_string_lossy().to_string(),
        genres: None, artist_names: None, folder: None,
    };
    db.upsert_track(&track).await.unwrap();

    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);
    let state = Arc::new(AppState {
        db, scan_handle, jwt_secret: JWT_SECRET.to_vec(), oidc: None,
    });

    let app = api::router(state.clone());
    let token = login_token(app, "u", "pass").await;

    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/tracks/1/stream")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"fake audio data");
}
