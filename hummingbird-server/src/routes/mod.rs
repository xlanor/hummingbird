pub mod albums;
pub mod artists;
pub mod art;
pub mod playlists;
pub mod scan;
pub mod search;
pub mod stream;
pub mod tracks;

use std::sync::Arc;
use axum::Router;

use crate::db::Repository;
use crate::scanner::ScanHandle;

pub struct AppState {
    pub repo: Arc<dyn Repository>,
    pub scan_handle: ScanHandle,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/v1", api_routes())
        .with_state(state)
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Albums
        .route("/albums", axum::routing::get(albums::list_albums))
        .route("/albums/{id}", axum::routing::get(albums::get_album))
        .route("/albums/{id}/tracks", axum::routing::get(albums::get_album_tracks))
        .route("/albums/{id}/art", axum::routing::get(art::get_album_art))
        .route("/albums/{id}/thumb", axum::routing::get(art::get_album_thumb))
        // Artists
        .route("/artists", axum::routing::get(artists::list_artists))
        .route("/artists/{id}", axum::routing::get(artists::get_artist))
        .route("/artists/{id}/albums", axum::routing::get(artists::get_artist_albums))
        // Tracks
        .route("/tracks", axum::routing::get(tracks::list_tracks))
        .route("/tracks/{id}", axum::routing::get(tracks::get_track))
        .route("/tracks/{id}/stream", axum::routing::get(stream::stream_track))
        // Search
        .route("/search", axum::routing::get(search::search))
        // Stats
        .route("/stats", axum::routing::get(tracks::get_stats))
        // Playlists
        .route("/playlists", axum::routing::get(playlists::list_playlists))
        .route("/playlists", axum::routing::post(playlists::create_playlist))
        .route("/playlists/{id}", axum::routing::get(playlists::get_playlist))
        .route("/playlists/{id}", axum::routing::delete(playlists::delete_playlist))
        .route("/playlists/{id}/tracks", axum::routing::post(playlists::add_track))
        .route("/playlists/{id}/tracks/{item_id}", axum::routing::delete(playlists::remove_track))
        .route("/playlists/{id}/tracks/{item_id}", axum::routing::put(playlists::move_track))
        // Scanner
        .route("/scan", axum::routing::post(scan::trigger_scan))
        .route("/scan/force", axum::routing::post(scan::force_scan))
        .route("/scan/status", axum::routing::get(scan::scan_status))
}
