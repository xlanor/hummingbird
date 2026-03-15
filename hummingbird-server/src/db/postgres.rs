use async_trait::async_trait;
use sqlx::postgres::PgPool;
use sqlx::Row;

use super::{Repository, Result};
use crate::models::*;

pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        let sql = include_str!("../../migrations/postgres/001_initial_schema.sql");
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl Repository for PostgresRepository {
    async fn list_albums(&self, sort: AlbumSort, order: SortOrder) -> Result<Vec<AlbumSummary>> {
        let order_clause = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        let order_by = match sort {
            AlbumSort::Title => format!("LOWER(al.title_sortable) {order_clause}"),
            AlbumSort::Artist => format!("LOWER(a.name) {order_clause}, LOWER(al.title_sortable) ASC"),
            AlbumSort::Release => format!("al.release_date {order_clause} NULLS LAST, LOWER(al.title_sortable) ASC"),
            AlbumSort::Label => format!("LOWER(al.label) {order_clause} NULLS LAST, LOWER(al.title_sortable) ASC"),
            AlbumSort::Catalog => format!("LOWER(al.catalog_number) {order_clause} NULLS LAST, LOWER(al.title_sortable) ASC"),
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
             CAST(release_date AS TEXT) as release_date, date_precision, \
             created_at, label, catalog_number, isrc, vinyl_numbering \
             FROM album WHERE id = $1",
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
             FROM track WHERE album_id = $1 \
             ORDER BY disc_number ASC NULLS FIRST, track_number ASC NULLS FIRST",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks)
    }

    async fn get_album_art(&self, id: i64) -> Result<Option<ArtBlob>> {
        let row: Option<(Option<Vec<u8>>, Option<String>)> =
            sqlx::query_as("SELECT image, image_mime FROM album WHERE id = $1")
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
            sqlx::query_as("SELECT thumb, image_mime FROM album WHERE id = $1")
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
            ArtistSort::Name => format!("LOWER(a.name_sortable) {order_clause}"),
            ArtistSort::Albums => format!("album_count {order_clause}, LOWER(a.name_sortable) ASC"),
            ArtistSort::Tracks => format!("track_count {order_clause}, LOWER(a.name_sortable) ASC"),
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
             FROM artist WHERE id = $1",
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
             WHERE al.artist_id = $1 \
             ORDER BY al.release_date ASC NULLS LAST",
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
            TrackSort::Title => format!("LOWER(t.title_sortable) {order_clause}"),
            TrackSort::Artist => format!("LOWER(t.artist_names) {order_clause} NULLS LAST, LOWER(t.title_sortable) ASC"),
            TrackSort::Album => format!("LOWER(al.title_sortable) {order_clause} NULLS LAST, t.disc_number ASC NULLS FIRST, t.track_number ASC NULLS FIRST"),
            TrackSort::Duration => format!("t.duration {order_clause}"),
            TrackSort::TrackNumber => format!("t.track_number {order_clause} NULLS LAST"),
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
             FROM track WHERE id = $1",
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
             FROM artist a WHERE a.name ILIKE $1 LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        let albums = sqlx::query_as::<_, AlbumSummary>(
            "SELECT al.id, al.title, al.artist_id, a.name as artist_name \
             FROM album al LEFT JOIN artist a ON al.artist_id = a.id \
             WHERE al.title ILIKE $1 LIMIT 20",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        let tracks = sqlx::query_as::<_, Track>(
            "SELECT id, title, title_sortable, album_id, track_number, disc_number, \
             duration, created_at, location, artist_names \
             FROM track WHERE title ILIKE $1 LIMIT 20",
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
             GROUP BY playlist.id, playlist.name, playlist.created_at, playlist.type",
        )
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

    async fn create_playlist(&self, name: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("INSERT INTO playlist (name) VALUES ($1) RETURNING id")
            .bind(name)
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
