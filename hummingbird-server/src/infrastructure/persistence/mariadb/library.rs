use async_trait::async_trait;
use sqlx::Row;

use super::MariaDbDatabase;
use crate::domain::library::dao::LibraryDao;
use crate::domain::library::*;
use crate::errors::AppError;

type Result<T> = std::result::Result<T, AppError>;

#[async_trait]
impl LibraryDao for MariaDbDatabase {
    async fn list_albums(&self, sort: AlbumSort, order: SortOrder) -> Result<Vec<AlbumSummary>> {
        let order_clause = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
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

    async fn list_artists(&self, sort: ArtistSort, order: SortOrder) -> Result<Vec<ArtistSummary>> {
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
