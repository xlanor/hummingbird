use async_trait::async_trait;

use super::PostgresDatabase;
use crate::domain::playlist::dao::PlaylistDao;
use crate::domain::playlist::*;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
impl PlaylistDao for PostgresDatabase {
    async fn list_playlists(&self, user_id: i64) -> Result<Vec<PlaylistWithCount>> {
        let playlists = sqlx::query_as::<_, PlaylistWithCount>(
            "SELECT playlist.id, playlist.name, playlist.created_at, playlist.type, \
             COUNT(playlist_item.id) as track_count \
             FROM playlist LEFT JOIN playlist_item ON playlist.id = playlist_item.playlist_id \
             WHERE playlist.user_id = $1 OR playlist.user_id IS NULL \
             GROUP BY playlist.id, playlist.name, playlist.created_at, playlist.type",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(playlists)
    }

    async fn get_playlist(&self, id: i64) -> Result<PlaylistDetail> {
        let playlist = sqlx::query_as::<_, Playlist>(
            "SELECT id, name, created_at, type FROM playlist WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let tracks = sqlx::query_as::<_, PlaylistTrack>(
            "SELECT pl.id as item_id, pl.track_id, t.album_id, pl.position \
             FROM playlist_item pl JOIN track t ON pl.track_id = t.id \
             WHERE pl.playlist_id = $1 ORDER BY pl.position ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(PlaylistDetail { playlist, tracks })
    }

    async fn create_playlist(&self, name: &str, user_id: i64) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("INSERT INTO playlist (name, user_id) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn delete_playlist(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM playlist_item WHERE playlist_id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM playlist WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO playlist_item (playlist_id, track_id, position) \
             VALUES ($1, $2, \
             COALESCE((SELECT position FROM playlist_item WHERE playlist_id = $1 ORDER BY position DESC LIMIT 1) + 1, 1)) \
             RETURNING id",
        )
        .bind(playlist_id)
        .bind(track_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn remove_from_playlist(&self, item_id: i64) -> Result<()> {
        let item: (i64,) =
            sqlx::query_as("SELECT position FROM playlist_item WHERE id = $1")
                .bind(item_id)
                .fetch_one(&self.pool)
                .await?;

        sqlx::query("UPDATE playlist_item SET position = position - 1 WHERE position > $1")
            .bind(item.0)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM playlist_item WHERE id = $1")
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn move_playlist_item(&self, item_id: i64, position: i32) -> Result<()> {
        let current: (i64,) =
            sqlx::query_as("SELECT position FROM playlist_item WHERE id = $1")
                .bind(item_id)
                .fetch_one(&self.pool)
                .await?;
        let current_pos = current.0 as i32;
        let new_pos = position;

        if new_pos < current_pos {
            sqlx::query(
                "UPDATE playlist_item SET position = position + 1 WHERE position >= $1 AND position < $2",
            )
            .bind(new_pos)
            .bind(current_pos)
            .execute(&self.pool)
            .await?;
        } else if new_pos > current_pos {
            sqlx::query(
                "UPDATE playlist_item SET position = position - 1 WHERE position <= $1 AND position > $2",
            )
            .bind(new_pos)
            .bind(current_pos)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query("UPDATE playlist_item SET position = $1 WHERE id = $2")
            .bind(new_pos)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_playlist_owner(&self, playlist_id: i64) -> Result<i64> {
        let row: (Option<i64>,) =
            sqlx::query_as("SELECT user_id FROM playlist WHERE id = $1")
                .bind(playlist_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0.unwrap_or(0))
    }
}
