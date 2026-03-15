use async_trait::async_trait;

use super::*;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
pub trait PlaylistDao: Send + Sync {
    async fn list_playlists(&self, user_id: i64) -> Result<Vec<PlaylistWithCount>>;
    async fn get_playlist(&self, id: i64) -> Result<PlaylistDetail>;
    async fn create_playlist(&self, name: &str, user_id: i64) -> Result<i64>;
    async fn delete_playlist(&self, id: i64) -> Result<()>;
    async fn add_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<i64>;
    async fn remove_from_playlist(&self, item_id: i64) -> Result<()>;
    async fn move_playlist_item(&self, item_id: i64, position: i32) -> Result<()>;
    async fn get_playlist_owner(&self, playlist_id: i64) -> Result<i64>;
}
