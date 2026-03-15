use async_trait::async_trait;

use super::MariaDbDatabase;
use crate::domain::library::Track;
use crate::domain::scanner::dao::ScannerDao;
use crate::domain::scanner::{ScannedAlbum, ScannedTrack};
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

fn make_sortable(name: &str) -> String {
    let lower = name.to_lowercase();
    if let Some(rest) = lower.strip_prefix("the ") {
        rest.to_string()
    } else {
        lower
    }
}

#[async_trait]
impl ScannerDao for MariaDbDatabase {
    async fn upsert_artist(&self, name: &str) -> Result<i64> {
        let sortable = make_sortable(name);

        sqlx::query(
            "INSERT INTO artist (name, name_sortable) VALUES (?, ?) \
             ON DUPLICATE KEY UPDATE name = VALUES(name)",
        )
        .bind(name)
        .bind(&sortable)
        .execute(&self.pool)
        .await?;

        let (id,): (i64,) = sqlx::query_as("SELECT id FROM artist WHERE name = ?")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn upsert_album(&self, album: &ScannedAlbum) -> Result<i64> {
        sqlx::query(
            "INSERT INTO album (title, title_sortable, artist_id, image, thumb, release_date, \
             date_precision, label, catalog_number, isrc, mbid, vinyl_numbering) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             title = VALUES(title), \
             title_sortable = VALUES(title_sortable), \
             artist_id = VALUES(artist_id), \
             image = VALUES(image), \
             thumb = VALUES(thumb), \
             release_date = VALUES(release_date), \
             date_precision = VALUES(date_precision), \
             label = VALUES(label), \
             catalog_number = VALUES(catalog_number), \
             isrc = VALUES(isrc), \
             mbid = VALUES(mbid), \
             vinyl_numbering = vinyl_numbering OR VALUES(vinyl_numbering)",
        )
        .bind(&album.title)
        .bind(&album.title_sortable)
        .bind(album.artist_id)
        .bind(&album.image)
        .bind(&album.thumb)
        .bind(&album.release_date)
        .bind(album.date_precision)
        .bind(&album.label)
        .bind(&album.catalog_number)
        .bind(&album.isrc)
        .bind(&album.mbid)
        .bind(album.vinyl_numbering)
        .execute(&self.pool)
        .await?;

        let (id,): (i64,) = sqlx::query_as(
            "SELECT id FROM album WHERE title = ? AND artist_id = ? AND mbid = ?",
        )
        .bind(&album.title)
        .bind(album.artist_id)
        .bind(&album.mbid)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn upsert_track(&self, track: &ScannedTrack) -> Result<i64> {
        sqlx::query(
            "INSERT INTO track (title, title_sortable, album_id, track_number, disc_number, \
             duration, location, genres, artist_names, folder) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
             title = VALUES(title), \
             title_sortable = VALUES(title_sortable), \
             album_id = VALUES(album_id), \
             track_number = VALUES(track_number), \
             disc_number = VALUES(disc_number), \
             duration = VALUES(duration), \
             location = VALUES(location), \
             genres = VALUES(genres), \
             artist_names = VALUES(artist_names), \
             folder = VALUES(folder)",
        )
        .bind(&track.title)
        .bind(&track.title_sortable)
        .bind(track.album_id)
        .bind(track.track_number)
        .bind(track.disc_number)
        .bind(track.duration)
        .bind(&track.location)
        .bind(&track.genres)
        .bind(&track.artist_names)
        .bind(&track.folder)
        .execute(&self.pool)
        .await?;

        let (id,): (i64,) = sqlx::query_as("SELECT id FROM track WHERE location = ?")
            .bind(&track.location)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn upsert_album_path(&self, album_id: i64, path: &str, disc_num: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO album_path (album_id, path, disc_num) VALUES (?, ?, ?) \
             ON DUPLICATE KEY UPDATE path = VALUES(path)",
        )
        .bind(album_id)
        .bind(path)
        .bind(disc_num)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_track(&self, location: &str) -> Result<()> {
        let track_info: Option<(Option<i64>, Option<String>, Option<i32>)> = sqlx::query_as(
            "SELECT album_id, folder, disc_number FROM track WHERE location = ?",
        )
        .bind(location)
        .fetch_optional(&self.pool)
        .await?;

        sqlx::query("DELETE FROM track WHERE location = ?")
            .bind(location)
            .execute(&self.pool)
            .await?;

        if let Some((album_id, folder, disc_number)) = track_info {
            self.cleanup_orphans_after_track_delete(album_id, folder.as_deref(), disc_number)
                .await?;
        }

        Ok(())
    }

    async fn get_track_by_path(&self, path: &str) -> Result<Option<Track>> {
        let track = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE location = ?",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(track)
    }
}
