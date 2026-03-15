use async_trait::async_trait;

use super::*;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
pub trait LibraryDao: Send + Sync {
    async fn list_albums(&self, sort: AlbumSort, order: SortOrder) -> Result<Vec<AlbumSummary>>;
    async fn get_album(&self, id: i64) -> Result<Album>;
    async fn get_album_tracks(&self, id: i64) -> Result<Vec<Track>>;
    async fn get_album_art(&self, id: i64) -> Result<Option<ArtBlob>>;
    async fn get_album_thumb(&self, id: i64) -> Result<Option<ArtBlob>>;
    async fn list_artists(&self, sort: ArtistSort, order: SortOrder) -> Result<Vec<ArtistSummary>>;
    async fn get_artist(&self, id: i64) -> Result<Artist>;
    async fn get_artist_albums(&self, id: i64) -> Result<Vec<AlbumSummary>>;
    async fn list_tracks(&self, sort: TrackSort, order: SortOrder) -> Result<Vec<Track>>;
    async fn get_track(&self, id: i64) -> Result<Track>;
    async fn search(&self, query: &str) -> Result<SearchResults>;
    async fn get_stats(&self) -> Result<LibraryStats>;
}
