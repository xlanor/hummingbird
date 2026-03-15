use async_trait::async_trait;
use sqlx::mysql::MySqlPool;
use sqlx::Row;

use super::{Repository, Result};
use crate::models::*;

pub struct MariaDbRepository {
    pool: MySqlPool,
}

impl MariaDbRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        // MariaDB doesn't support multiple statements in a single query via sqlx,
        // so we split and execute each statement individually.
        let sql = include_str!("../../migrations/mariadb/001_initial_schema.sql");
        for stmt in sql.split(';') {
            let trimmed = stmt.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("--") {
                sqlx::raw_sql(&format!("{trimmed};"))
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    /// MariaDB doesn't have trigger-based cascade cleanup like SQLite/Postgres,
    /// so we do it in the application layer after track deletes.
    async fn cleanup_orphans_after_track_delete(
        &self,
        album_id: Option<i64>,
        folder: Option<&str>,
        disc_number: Option<i32>,
    ) -> Result<()> {
        // Clean up album_path
        if let (Some(aid), Some(f)) = (album_id, folder) {
            let disc = disc_number.unwrap_or(-1);
            sqlx::query(
                "DELETE FROM album_path \
                 WHERE path = ? AND disc_num = ? AND album_id = ? \
                 AND NOT EXISTS (SELECT 1 FROM track WHERE folder = ? AND disc_number <=> ? AND album_id = ?)",
            )
            .bind(f)
            .bind(disc)
            .bind(aid)
            .bind(f)
            .bind(disc_number)
            .bind(aid)
            .execute(&self.pool)
            .await?;
        }

        // Clean up empty album
        if let Some(aid) = album_id {
            let deleted = sqlx::query(
                "DELETE FROM album WHERE id = ? AND NOT EXISTS (SELECT 1 FROM track WHERE album_id = ?)",
            )
            .bind(aid)
            .bind(aid)
            .execute(&self.pool)
            .await?;

            if deleted.rows_affected() > 0 {
                // Also clean up album_path for this album
                sqlx::query("DELETE FROM album_path WHERE album_id = ?")
                    .bind(aid)
                    .execute(&self.pool)
                    .await
                    .ok();
            }
        }

        // Clean up orphaned artists
        sqlx::query("DELETE FROM artist WHERE NOT EXISTS (SELECT 1 FROM album WHERE artist_id = artist.id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Repository for MariaDbRepository {
    async fn list_albums(&self, sort: AlbumSort, order: SortOrder) -> Result<Vec<AlbumSummary>> {
        let order_clause = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        // MariaDB uses utf8mb4_unicode_ci collation set at table level, so LOWER() for sort
        let order_by = match sort {
            AlbumSort::Title => format!("al.title_sortable {order_clause}"),
            AlbumSort::Artist => format!("a.name {order_clause}, al.title_sortable ASC"),
            AlbumSort::Release => format!("al.release_date {order_clause}, al.title_sortable ASC"),
            AlbumSort::Label => format!("al.label {order_clause}, al.title_sortable ASC"),
            AlbumSort::Catalog => format!("al.catalog_number {order_clause}, al.title_sortable ASC"),
        };

        let query = format!(
            "SELECT al.id, al.title, al.artist_id, a.name as artist_name \
             FROM album al LEFT JOIN artist a ON al.artist_id = a.id \
             ORDER BY {order_by}"
        );

        let rows = sqlx::query_as::<_, AlbumSummary>(&query)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn get_album(&self, id: i64) -> Result<Album> {
        let album = sqlx::query_as::<_, Album>(
            "SELECT id, title, title_sortable, artist_id, \
             CAST(release_date AS CHAR) as release_date, date_precision, \
             created_at, label, catalog_number, isrc, vinyl_numbering \
             FROM album WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(album)
    }

    async fn get_album_tracks(&self, id: i64) -> Result<Vec<Track>> {
        let tracks = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE album_id = ? \
             ORDER BY disc_number ASC, track_number ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks)
    }

    async fn get_album_art(&self, id: i64) -> Result<Option<ArtBlob>> {
        let row: Option<(Option<Vec<u8>>, Option<String>)> =
            sqlx::query_as("SELECT image, image_mime FROM album WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some((Some(data), mime)) => Ok(Some(ArtBlob { data, mime })),
            _ => Ok(None),
        }
    }

    async fn get_album_thumb(&self, id: i64) -> Result<Option<ArtBlob>> {
        let row: Option<(Option<Vec<u8>>, Option<String>)> =
            sqlx::query_as("SELECT thumb, image_mime FROM album WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some((Some(data), mime)) => Ok(Some(ArtBlob { data, mime })),
            _ => Ok(None),
        }
    }

    async fn list_artists(
        &self,
        sort: ArtistSort,
        order: SortOrder,
    ) -> Result<Vec<ArtistSummary>> {
        let order_clause = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        let order_by = match sort {
            ArtistSort::Name => format!("a.name_sortable {order_clause}"),
            ArtistSort::Albums => format!("album_count {order_clause}, a.name_sortable ASC"),
            ArtistSort::Tracks => format!("track_count {order_clause}, a.name_sortable ASC"),
        };

        let query = format!(
            "SELECT a.id, a.name, \
             (SELECT COUNT(*) FROM album WHERE artist_id = a.id) AS album_count, \
             (SELECT COUNT(*) FROM track t JOIN album al ON t.album_id = al.id WHERE al.artist_id = a.id) AS track_count \
             FROM artist a \
             ORDER BY {order_by}"
        );

        let rows = sqlx::query_as::<_, ArtistSummary>(&query)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn get_artist(&self, id: i64) -> Result<Artist> {
        let artist = sqlx::query_as::<_, Artist>(
            "SELECT id, name, name_sortable, bio, created_at, image_mime \
             FROM artist WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(artist)
    }

    async fn get_artist_albums(&self, id: i64) -> Result<Vec<AlbumSummary>> {
        let albums = sqlx::query_as::<_, AlbumSummary>(
            "SELECT al.id, al.title, al.artist_id, a.name as artist_name \
             FROM album al LEFT JOIN artist a ON al.artist_id = a.id \
             WHERE al.artist_id = ? \
             ORDER BY al.release_date ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(albums)
    }

    async fn list_tracks(&self, sort: TrackSort, order: SortOrder) -> Result<Vec<Track>> {
        let order_clause = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        let order_by = match sort {
            TrackSort::Title => format!("t.title_sortable {order_clause}"),
            TrackSort::Artist => format!("t.artist_names {order_clause}, t.title_sortable ASC"),
            TrackSort::Album => format!("al.title_sortable {order_clause}, t.disc_number ASC, t.track_number ASC"),
            TrackSort::Duration => format!("t.duration {order_clause}"),
            TrackSort::TrackNumber => format!("t.track_number {order_clause}"),
        };

        let query = format!(
            "SELECT t.id, t.title, t.title_sortable, t.album_id, t.track_number, \
             t.disc_number, t.duration, t.created_at, t.location, t.artist_names \
             FROM track t LEFT JOIN album al ON t.album_id = al.id \
             ORDER BY {order_by}"
        );

        let rows = sqlx::query_as::<_, Track>(&query)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn get_track(&self, id: i64) -> Result<Track> {
        let track = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(track)
    }

    async fn search(&self, query: &str) -> Result<SearchResults> {
        let pattern = format!("%{query}%");

        let artists = sqlx::query_as::<_, ArtistSummary>(
            "SELECT a.id, a.name, \
             (SELECT COUNT(*) FROM album WHERE artist_id = a.id) AS album_count, \
             (SELECT COUNT(*) FROM track t JOIN album al ON t.album_id = al.id WHERE al.artist_id = a.id) AS track_count \
             FROM artist a WHERE a.name LIKE ? LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        let albums = sqlx::query_as::<_, AlbumSummary>(
            "SELECT al.id, al.title, al.artist_id, a.name as artist_name \
             FROM album al LEFT JOIN artist a ON al.artist_id = a.id \
             WHERE al.title LIKE ? LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        let tracks = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE title LIKE ? LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(SearchResults {
            artists,
            albums,
            tracks,
        })
    }

    async fn list_playlists(&self) -> Result<Vec<PlaylistWithCount>> {
        let playlists = sqlx::query_as::<_, PlaylistWithCount>(
            "SELECT playlist.id, playlist.name, playlist.created_at, playlist.type, \
             COUNT(playlist_item.id) as track_count \
             FROM playlist LEFT JOIN playlist_item ON playlist.id = playlist_item.playlist_id \
             GROUP BY playlist.id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(playlists)
    }

    async fn get_playlist(&self, id: i64) -> Result<PlaylistDetail> {
        let playlist = sqlx::query_as::<_, Playlist>(
            "SELECT id, name, created_at, type FROM playlist WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let tracks = sqlx::query_as::<_, PlaylistTrack>(
            "SELECT pl.id as item_id, pl.track_id, t.album_id, pl.position \
             FROM playlist_item pl JOIN track t ON pl.track_id = t.id \
             WHERE pl.playlist_id = ? ORDER BY pl.position ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(PlaylistDetail { playlist, tracks })
    }

    async fn create_playlist(&self, name: &str) -> Result<i64> {
        let result = sqlx::query("INSERT INTO playlist (name) VALUES (?)")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(result.last_insert_id() as i64)
    }

    async fn delete_playlist(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM playlist_item WHERE playlist_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM playlist WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO playlist_item (playlist_id, track_id, position) \
             VALUES (?, ?, \
             COALESCE((SELECT max_pos FROM (SELECT MAX(position) as max_pos FROM playlist_item WHERE playlist_id = ?) t) + 1, 1))",
        )
        .bind(playlist_id)
        .bind(track_id)
        .bind(playlist_id)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i64)
    }

    async fn remove_from_playlist(&self, item_id: i64) -> Result<()> {
        let item: (i64,) =
            sqlx::query_as("SELECT position FROM playlist_item WHERE id = ?")
                .bind(item_id)
                .fetch_one(&self.pool)
                .await?;

        sqlx::query("UPDATE playlist_item SET position = position - 1 WHERE position > ?")
            .bind(item.0)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM playlist_item WHERE id = ?")
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn move_playlist_item(&self, item_id: i64, position: i32) -> Result<()> {
        let current: (i64,) =
            sqlx::query_as("SELECT position FROM playlist_item WHERE id = ?")
                .bind(item_id)
                .fetch_one(&self.pool)
                .await?;
        let current_pos = current.0 as i32;
        let new_pos = position;

        if new_pos < current_pos {
            sqlx::query(
                "UPDATE playlist_item SET position = position + 1 WHERE position >= ? AND position < ?",
            )
            .bind(new_pos)
            .bind(current_pos)
            .execute(&self.pool)
            .await?;
        } else if new_pos > current_pos {
            sqlx::query(
                "UPDATE playlist_item SET position = position - 1 WHERE position <= ? AND position > ?",
            )
            .bind(new_pos)
            .bind(current_pos)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query("UPDATE playlist_item SET position = ? WHERE id = ?")
            .bind(new_pos)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn upsert_artist(&self, name: &str) -> Result<i64> {
        let sortable = make_sortable(name);

        // MariaDB: INSERT ... ON DUPLICATE KEY UPDATE, then fetch id
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

        // Fetch the id (LAST_INSERT_ID is 0 on update)
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
        // Fetch track info before delete for orphan cleanup
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

        // Application-level cascade cleanup for MariaDB
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

    async fn get_stats(&self) -> Result<LibraryStats> {
        let row = sqlx::query("SELECT COUNT(*) as track_count, COALESCE(SUM(duration), 0) as total_duration FROM track")
            .fetch_one(&self.pool)
            .await?;
        Ok(LibraryStats {
            track_count: row.get("track_count"),
            total_duration: row.get("total_duration"),
        })
    }
}

fn make_sortable(name: &str) -> String {
    let lower = name.to_lowercase();
    if let Some(rest) = lower.strip_prefix("the ") {
        rest.to_string()
    } else {
        lower
    }
}
