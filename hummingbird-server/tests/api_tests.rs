use std::sync::Arc;

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use http_body_util::BodyExt;
use jsonwebtoken::jwk::JwkSet;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower::ServiceExt;

use hummingbird_server::api::{self, AppState};
use hummingbird_server::domain::scanner::dao::ScannerDao;
use hummingbird_server::domain::scanner::orchestrator;
use hummingbird_server::domain::scanner::{ScannedAlbum, ScannedTrack};
use hummingbird_server::domain::user::dao::UserDao;
use hummingbird_server::infrastructure::auth;
use hummingbird_server::infrastructure::auth::OidcConfig;
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
        oidc_only: false,
        public_url: None,
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
        oidc_only: false,
        public_url: None,
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
    json["access_token"].as_str().unwrap().to_string()
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
        oidc_only: false, public_url: None,
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

// ---------------------------------------------------------------------------
// New OIDC / token tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_providers_endpoint() {
    let (app, _) = setup_app().await;
    let resp = app
        .oneshot(Request::builder().uri("/api/v1/auth/providers").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["password"], true);
    assert!(json.get("oidc").is_none() || json["oidc"].is_null());
}

#[tokio::test]
async fn test_providers_endpoint_oidc_only() {
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

    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);
    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: None,
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let app = api::router(state);
    let resp = app
        .oneshot(Request::builder().uri("/api/v1/auth/providers").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["password"], false);
    assert_eq!(json["oidc"]["enabled"], true);
    assert!(json["oidc"]["authorize_url"].as_str().unwrap().contains("/auth/oidc/authorize"));
}

#[tokio::test]
async fn test_login_disabled_when_oidc_only() {
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
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "username": "testadmin",
                        "password": "testpass",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_refresh_token_flow() {
    let (_, _, state) = setup_seeded_app().await;

    // Login to get token pair
    let app = api::router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "username": "testadmin",
                        "password": "testpass",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Use refresh token to get new pair
    let app = api::router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "refresh_token": refresh_token,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let refresh_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(refresh_json["access_token"].as_str().is_some());
    assert!(refresh_json["refresh_token"].as_str().is_some());
    assert!(refresh_json["expires_in"].as_u64().unwrap() > 0);

    // Verify new access token works
    let new_access = refresh_json["access_token"].as_str().unwrap();
    let app = api::router(state.clone());
    let (status, _) = authed_get(app, "/api/v1/auth/me", new_access).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_refresh_token_rejected_as_access() {
    let (_, _, state) = setup_seeded_app().await;

    // Login to get token pair
    let app = api::router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "username": "testadmin",
                        "password": "testpass",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let refresh_token = login_json["refresh_token"].as_str().unwrap();

    // Try to use refresh token as Bearer auth — should fail
    let app = api::router(state.clone());
    let (status, _) = authed_get(app, "/api/v1/auth/me", refresh_token).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_returns_token_pair() {
    let (_, _, state) = setup_seeded_app().await;
    let app = api::router(state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .method(http::Method::POST)
                .uri("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "username": "testadmin",
                        "password": "testpass",
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
    assert!(json["access_token"].as_str().is_some());
    assert!(json["refresh_token"].as_str().is_some());
    assert!(json["expires_in"].as_u64().is_some());
    assert!(json["user"]["username"].as_str().is_some());
}

// ---------------------------------------------------------------------------
// OIDC authorize / callback integration tests
// ---------------------------------------------------------------------------

/// Build a fake unsigned JWT with the given claims (for use as a mock id_token).
fn fake_id_token(claims: &serde_json::Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap());
    format!("{header}.{payload}.fakesig")
}

/// Create an OidcConfig that points to a local mock server.
fn mock_oidc_config(
    token_endpoint: &str,
    authorization_endpoint: &str,
    role_claim: &str,
    admin_group: Option<&str>,
) -> OidcConfig {
    OidcConfig {
        issuer: "https://fake-idp.example.com".into(),
        audience: "test-client".into(),
        jwks: Arc::new(RwLock::new(JwkSet { keys: vec![] })),
        authorization_endpoint: Some(authorization_endpoint.into()),
        token_endpoint: Some(token_endpoint.into()),
        client_id: Some("test-client".into()),
        client_secret: Some("test-secret".into()),
        role_claim: role_claim.into(),
        admin_group: admin_group.map(String::from),
    }
}

/// Create an in-memory SQLite DB with migrations applied.
async fn setup_db() -> SqliteDatabase {
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
    db
}

/// Start a mock IdP token endpoint that returns the given id_token.
/// Returns the base URL (e.g. "http://127.0.0.1:PORT").
async fn start_mock_idp(id_token: String) -> String {
    use axum::routing::post;

    let app = axum::Router::new().route(
        "/token",
        post(move || {
            let token = id_token.clone();
            async move {
                axum::Json(serde_json::json!({
                    "access_token": "mock-access-token",
                    "token_type": "Bearer",
                    "id_token": token,
                }))
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{}", addr.port())
}

/// Build a valid PKCE state JWT signed with the test secret.
fn build_state_jwt(code_verifier: &str, redirect_uri: &str) -> String {
    #[derive(Serialize)]
    struct PkceState {
        code_verifier: String,
        redirect_uri: String,
        exp: usize,
    }

    let now = chrono::Utc::now().timestamp() as usize;
    let state = PkceState {
        code_verifier: code_verifier.into(),
        redirect_uri: redirect_uri.into(),
        exp: now + 600,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &state,
        &jsonwebtoken::EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap()
}

#[tokio::test]
async fn test_oidc_authorize_redirect() {
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        "http://localhost/token",
        "https://fake-idp.example.com/authorize",
        "groups",
        None,
    );

    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/authorize?redirect_uri=https://music.example.com/callback")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be a 307 temporary redirect
    assert!(
        resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER,
        "expected redirect, got {}",
        resp.status()
    );

    let location = resp.headers().get("location").unwrap().to_str().unwrap();

    // Verify the redirect URL contains expected PKCE and OAuth params
    assert!(location.starts_with("https://fake-idp.example.com/authorize?"));
    assert!(location.contains("response_type=code"));
    assert!(location.contains("client_id=test-client"));
    assert!(location.contains("code_challenge="));
    assert!(location.contains("code_challenge_method=S256"));
    assert!(location.contains("state="));
    assert!(location.contains("scope=openid"));
    assert!(location.contains("redirect_uri="));
}

#[tokio::test]
async fn test_oidc_authorize_uses_default_redirect() {
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        "http://localhost/token",
        "https://fake-idp.example.com/authorize",
        "groups",
        None,
    );

    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    // No redirect_uri query param — should default to public_url
    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/authorize")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER,
    );

    // Decode the state JWT from the redirect URL to verify default redirect_uri
    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    let url = url::Url::parse(location).unwrap();
    let state_param = url.query_pairs().find(|(k, _)| k == "state").unwrap().1;
    let decoded_state = urlencoding::decode(&state_param).unwrap();

    #[derive(Deserialize)]
    struct PkceState { redirect_uri: String }

    let mut validation = jsonwebtoken::Validation::default();
    validation.insecure_disable_signature_validation();
    validation.set_required_spec_claims::<&str>(&[]);
    let data = jsonwebtoken::decode::<PkceState>(
        &decoded_state,
        &jsonwebtoken::DecodingKey::from_secret(JWT_SECRET),
        &validation,
    )
    .unwrap();

    assert_eq!(data.claims.redirect_uri, "https://music.example.com/");
}

#[tokio::test]
async fn test_oidc_callback_creates_user_and_redirects() {
    let id_token = fake_id_token(&serde_json::json!({
        "sub": "oidc-user-123",
        "preferred_username": "jdoe",
        "name": "Jane Doe",
        "email": "jdoe@example.com",
    }));

    let mock_url = start_mock_idp(id_token).await;
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        &format!("{mock_url}/token"),
        "https://fake-idp.example.com/authorize",
        "groups",
        None,
    );

    let state = Arc::new(AppState {
        db: db.clone(),
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let state_jwt = build_state_jwt("test-verifier", "https://music.example.com/app");

    let app = api::router(state.clone());
    let callback_uri = format!(
        "/api/v1/auth/oidc/callback?code=test-auth-code&state={}",
        urlencoding::encode(&state_jwt),
    );
    let resp = app
        .oneshot(Request::builder().uri(&callback_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should redirect to the frontend with tokens in fragment
    assert!(
        resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER,
        "expected redirect, got {}",
        resp.status()
    );

    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.starts_with("https://music.example.com/app#"));
    assert!(location.contains("access_token="));
    assert!(location.contains("refresh_token="));
    assert!(location.contains("expires_in="));

    // Verify user was created in DB
    let user = db
        .get_user_by_oidc("https://fake-idp.example.com", "oidc-user-123")
        .await
        .unwrap();
    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.username, "jdoe");
    assert_eq!(user.display_name.as_deref(), Some("Jane Doe"));
    assert_eq!(user.role, "user"); // no admin group configured

    // Extract the access token from the redirect and verify it works
    let fragment = location.split_once('#').unwrap().1;
    let params: std::collections::HashMap<String, String> = url::form_urlencoded::parse(fragment.as_bytes())
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let access_token = &params["access_token"];

    let app = api::router(state.clone());
    let (status, json) = authed_get(app, "/api/v1/auth/me", access_token).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["username"], "jdoe");
}

#[tokio::test]
async fn test_oidc_callback_maps_admin_role() {
    let id_token = fake_id_token(&serde_json::json!({
        "sub": "admin-user-456",
        "preferred_username": "adminuser",
        "name": "Admin User",
        "groups": ["users", "hummingbird-admins"],
    }));

    let mock_url = start_mock_idp(id_token).await;
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        &format!("{mock_url}/token"),
        "https://fake-idp.example.com/authorize",
        "groups",
        Some("hummingbird-admins"),
    );

    let state = Arc::new(AppState {
        db: db.clone(),
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let state_jwt = build_state_jwt("verifier", "https://music.example.com/");

    let app = api::router(state.clone());
    let callback_uri = format!(
        "/api/v1/auth/oidc/callback?code=authcode&state={}",
        urlencoding::encode(&state_jwt),
    );
    let resp = app
        .oneshot(Request::builder().uri(&callback_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert!(
        resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER,
    );

    // Verify user was created with admin role
    let user = db
        .get_user_by_oidc("https://fake-idp.example.com", "admin-user-456")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.username, "adminuser");
    assert_eq!(user.role, "admin");
}

#[tokio::test]
async fn test_oidc_callback_updates_role_on_subsequent_login() {
    // First login: user is NOT in admin group → gets "user" role
    let id_token_user = fake_id_token(&serde_json::json!({
        "sub": "role-change-789",
        "preferred_username": "morpheus",
        "name": "Morpheus",
        "groups": ["users"],
    }));

    let mock_url = start_mock_idp(id_token_user).await;
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        &format!("{mock_url}/token"),
        "https://fake-idp.example.com/authorize",
        "groups",
        Some("admins"),
    );

    let app_state = Arc::new(AppState {
        db: db.clone(),
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let state_jwt = build_state_jwt("v1", "https://music.example.com/");
    let app = api::router(app_state.clone());
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!(
                    "/api/v1/auth/oidc/callback?code=c1&state={}",
                    urlencoding::encode(&state_jwt),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER);

    let user = db.get_user_by_oidc("https://fake-idp.example.com", "role-change-789").await.unwrap().unwrap();
    assert_eq!(user.role, "user");

    // Second login: user IS now in admin group → role should update to "admin"
    let id_token_admin = fake_id_token(&serde_json::json!({
        "sub": "role-change-789",
        "preferred_username": "morpheus",
        "name": "Morpheus",
        "groups": ["users", "admins"],
    }));

    let mock_url2 = start_mock_idp(id_token_admin).await;

    // Update oidc config to point to new mock
    let oidc2 = mock_oidc_config(
        &format!("{mock_url2}/token"),
        "https://fake-idp.example.com/authorize",
        "groups",
        Some("admins"),
    );

    let scan_handle2 = orchestrator::start_scanner(db.clone(), vec![]);
    let app_state2 = Arc::new(AppState {
        db: db.clone(),
        scan_handle: scan_handle2,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc2),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let state_jwt2 = build_state_jwt("v2", "https://music.example.com/");
    let app = api::router(app_state2);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!(
                    "/api/v1/auth/oidc/callback?code=c2&state={}",
                    urlencoding::encode(&state_jwt2),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER);

    let user = db.get_user_by_oidc("https://fake-idp.example.com", "role-change-789").await.unwrap().unwrap();
    assert_eq!(user.role, "admin");
}

#[tokio::test]
async fn test_oidc_callback_nested_role_claim() {
    let id_token = fake_id_token(&serde_json::json!({
        "sub": "nested-claim-user",
        "preferred_username": "nested",
        "name": "Nested User",
        "realm_access": {
            "roles": ["offline_access", "super-admin"]
        },
    }));

    let mock_url = start_mock_idp(id_token).await;
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        &format!("{mock_url}/token"),
        "https://fake-idp.example.com/authorize",
        "realm_access.roles",
        Some("super-admin"),
    );

    let state = Arc::new(AppState {
        db: db.clone(),
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let state_jwt = build_state_jwt("v", "https://music.example.com/");
    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri(&format!(
                    "/api/v1/auth/oidc/callback?code=c&state={}",
                    urlencoding::encode(&state_jwt),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(resp.status() == StatusCode::TEMPORARY_REDIRECT || resp.status() == StatusCode::SEE_OTHER);

    let user = db.get_user_by_oidc("https://fake-idp.example.com", "nested-claim-user").await.unwrap().unwrap();
    assert_eq!(user.role, "admin");
}

#[tokio::test]
async fn test_oidc_callback_invalid_state_rejected() {
    let db = setup_db().await;
    let db: Arc<dyn Database> = Arc::new(db);
    let scan_handle = orchestrator::start_scanner(db.clone(), vec![]);

    let oidc = mock_oidc_config(
        "http://localhost/token",
        "https://fake-idp.example.com/authorize",
        "groups",
        None,
    );

    let state = Arc::new(AppState {
        db,
        scan_handle,
        jwt_secret: JWT_SECRET.to_vec(),
        oidc: Some(oidc),
        oidc_only: true,
        public_url: Some("https://music.example.com".into()),
    });

    let app = api::router(state);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/callback?code=test&state=invalid-jwt-garbage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
