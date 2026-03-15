use async_trait::async_trait;

use super::PostgresDatabase;
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
impl ScannerDao for PostgresDatabase {
    async fn upsert_artist(&self, name: &str) -> Result<i64> {
        let sortable = make_sortable(name);

        let row: (i64,) = sqlx::query_as(
            "INSERT INTO artist (name, name_sortable) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name \
             RETURNING id",
        )
        .bind(name)
        .bind(&sortable)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn upsert_album(&self, album: &ScannedAlbum) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO album (title, title_sortable, artist_id, image, thumb, release_date, \
             date_precision, label, catalog_number, isrc, mbid, vinyl_numbering) \
             VALUES ($1, $2, $3, $4, $5, CAST($6 AS DATE), $7, $8, $9, $10, $11, $12) \
             ON CONFLICT (title, artist_id, mbid) DO UPDATE SET \
             title = EXCLUDED.title, \
             title_sortable = EXCLUDED.title_sortable, \
             artist_id = EXCLUDED.artist_id, \
             image = EXCLUDED.image, \
             thumb = EXCLUDED.thumb, \
             release_date = EXCLUDED.release_date, \
             date_precision = EXCLUDED.date_precision, \
             label = EXCLUDED.label, \
             catalog_number = EXCLUDED.catalog_number, \
             isrc = EXCLUDED.isrc, \
             mbid = EXCLUDED.mbid, \
             vinyl_numbering = album.vinyl_numbering OR EXCLUDED.vinyl_numbering \
             RETURNING id",
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
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn upsert_track(&self, track: &ScannedTrack) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO track (title, title_sortable, album_id, track_number, disc_number, \
             duration, location, genres, artist_names, folder) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (location) DO UPDATE SET \
             title = EXCLUDED.title, \
             title_sortable = EXCLUDED.title_sortable, \
             album_id = EXCLUDED.album_id, \
             track_number = EXCLUDED.track_number, \
             disc_number = EXCLUDED.disc_number, \
             duration = EXCLUDED.duration, \
             location = EXCLUDED.location, \
             genres = EXCLUDED.genres, \
             artist_names = EXCLUDED.artist_names, \
             folder = EXCLUDED.folder \
             RETURNING id",
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
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn upsert_album_path(&self, album_id: i64, path: &str, disc_num: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO album_path (album_id, path, disc_num) VALUES ($1, $2, $3) \
             ON CONFLICT (album_id, disc_num) DO NOTHING",
        )
        .bind(album_id)
        .bind(path)
        .bind(disc_num)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_track(&self, location: &str) -> Result<()> {
        sqlx::query("DELETE FROM track WHERE location = $1")
            .bind(location)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_track_by_path(&self, path: &str) -> Result<Option<Track>> {
        let track = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE location = $1",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(track)
    }
}
