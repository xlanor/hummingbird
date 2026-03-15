pub mod albums;
pub mod art;
pub mod artists;
pub mod auth;
pub mod playlists;
pub mod scan;
pub mod search;
pub mod stream;
pub mod tracks;

use std::sync::Arc;
use axum::Router;
use axum::middleware;

use crate::domain::scanner::ScanHandle;
use crate::infrastructure::auth::OidcConfig;
use crate::infrastructure::persistence::Database;

pub struct AppState {
    pub db: Arc<dyn Database>,
    pub scan_handle: ScanHandle,
    pub jwt_secret: Vec<u8>,
    pub oidc: Option<OidcConfig>,
}

pub fn router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/auth/login", axum::routing::post(auth::login));

    let protected = Router::new()
        .route("/auth/me", axum::routing::get(auth::me))
        .route("/auth/password", axum::routing::put(auth::change_password))
        .route("/users", axum::routing::get(auth::list_users))
        .route("/users", axum::routing::post(auth::create_user))
        .route("/users/{id}", axum::routing::delete(auth::delete_user))
        .route("/albums", axum::routing::get(albums::list_albums))
        .route("/albums/{id}", axum::routing::get(albums::get_album))
        .route("/albums/{id}/tracks", axum::routing::get(albums::get_album_tracks))
        .route("/albums/{id}/art", axum::routing::get(art::get_album_art))
        .route("/albums/{id}/thumb", axum::routing::get(art::get_album_thumb))
        .route("/artists", axum::routing::get(artists::list_artists))
        .route("/artists/{id}", axum::routing::get(artists::get_artist))
        .route("/artists/{id}/albums", axum::routing::get(artists::get_artist_albums))
        .route("/tracks", axum::routing::get(tracks::list_tracks))
        .route("/tracks/{id}", axum::routing::get(tracks::get_track))
        .route("/tracks/{id}/stream", axum::routing::get(stream::stream_track))
        .route("/search", axum::routing::get(search::search))
        .route("/stats", axum::routing::get(tracks::get_stats))
        .route("/playlists", axum::routing::get(playlists::list_playlists))
        .route("/playlists", axum::routing::post(playlists::create_playlist))
        .route("/playlists/{id}", axum::routing::get(playlists::get_playlist))
        .route("/playlists/{id}", axum::routing::delete(playlists::delete_playlist))
        .route("/playlists/{id}/tracks", axum::routing::post(playlists::add_track))
        .route("/playlists/{id}/tracks/{item_id}", axum::routing::delete(playlists::remove_track))
        .route("/playlists/{id}/tracks/{item_id}", axum::routing::put(playlists::move_track))
        .route("/scan", axum::routing::post(scan::trigger_scan))
        .route("/scan/force", axum::routing::post(scan::force_scan))
        .route("/scan/status", axum::routing::get(scan::scan_status))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::infrastructure::auth::require_auth,
        ));

    Router::new()
        .nest("/api/v1", public.merge(protected))
        .with_state(state)
}
