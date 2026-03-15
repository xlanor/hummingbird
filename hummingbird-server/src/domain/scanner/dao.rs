use async_trait::async_trait;

use super::{ScannedAlbum, ScannedTrack, Track};
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
pub trait ScannerDao: Send + Sync {
    async fn upsert_artist(&self, name: &str) -> Result<i64>;
    async fn upsert_album(&self, album: &ScannedAlbum) -> Result<i64>;
    async fn upsert_track(&self, track: &ScannedTrack) -> Result<i64>;
    async fn upsert_album_path(&self, album_id: i64, path: &str, disc_num: i32) -> Result<()>;
    async fn delete_track(&self, location: &str) -> Result<()>;
    async fn get_track_by_path(&self, path: &str) -> Result<Option<Track>>;
}
