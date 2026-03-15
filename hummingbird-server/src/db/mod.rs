pub mod sqlite;

use async_trait::async_trait;

use crate::errors::AppError;
use crate::models::*;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
pub trait Repository: Send + Sync {
    // Albums
    async fn list_albums(&self, sort: AlbumSort, order: SortOrder) -> Result<Vec<AlbumSummary>>;
    async fn get_album(&self, id: i64) -> Result<Album>;
    async fn get_album_tracks(&self, id: i64) -> Result<Vec<Track>>;
    async fn get_album_art(&self, id: i64) -> Result<Option<ArtBlob>>;
    async fn get_album_thumb(&self, id: i64) -> Result<Option<ArtBlob>>;

    // Artists
    async fn list_artists(&self, sort: ArtistSort, order: SortOrder)
        -> Result<Vec<ArtistSummary>>;
    async fn get_artist(&self, id: i64) -> Result<Artist>;
    async fn get_artist_albums(&self, id: i64) -> Result<Vec<AlbumSummary>>;

    // Tracks
    async fn list_tracks(&self, sort: TrackSort, order: SortOrder) -> Result<Vec<Track>>;
    async fn get_track(&self, id: i64) -> Result<Track>;

    // Search
    async fn search(&self, query: &str) -> Result<SearchResults>;

    // Playlists
    async fn list_playlists(&self) -> Result<Vec<PlaylistWithCount>>;
    async fn get_playlist(&self, id: i64) -> Result<PlaylistDetail>;
    async fn create_playlist(&self, name: &str) -> Result<i64>;
    async fn delete_playlist(&self, id: i64) -> Result<()>;
    async fn add_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<i64>;
    async fn remove_from_playlist(&self, item_id: i64) -> Result<()>;
    async fn move_playlist_item(&self, item_id: i64, position: i32) -> Result<()>;

    // Scanner
    async fn upsert_artist(&self, name: &str) -> Result<i64>;
    async fn upsert_album(&self, album: &ScannedAlbum) -> Result<i64>;
    async fn upsert_track(&self, track: &ScannedTrack) -> Result<i64>;
    async fn upsert_album_path(&self, album_id: i64, path: &str, disc_num: i32) -> Result<()>;
    async fn delete_track(&self, location: &str) -> Result<()>;
    async fn get_track_by_path(&self, path: &str) -> Result<Option<Track>>;

    // Stats
    async fn get_stats(&self) -> Result<LibraryStats>;
}
