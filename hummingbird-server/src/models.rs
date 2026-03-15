use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub name_sortable: String,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub image_mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ArtistSummary {
    pub id: i64,
    pub name: String,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub title_sortable: String,
    pub artist_id: Option<i64>,
    pub release_date: Option<String>,
    pub date_precision: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub isrc: Option<String>,
    pub vinyl_numbering: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AlbumSummary {
    pub id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub title_sortable: String,
    pub album_id: Option<i64>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration: i64,
    pub created_at: DateTime<Utc>,
    pub location: String,
    pub artist_names: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "type")]
    pub playlist_type: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlaylistWithCount {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    #[sqlx(rename = "type")]
    pub playlist_type: i32,
    pub track_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlaylistItem {
    pub id: i64,
    pub playlist_id: i64,
    pub track_id: i64,
    pub created_at: DateTime<Utc>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistDetail {
    #[serde(flatten)]
    pub playlist: Playlist,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlaylistTrack {
    pub item_id: i64,
    pub track_id: i64,
    pub album_id: Option<i64>,
    pub position: i64,
}

pub struct ArtBlob {
    pub data: Vec<u8>,
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LibraryStats {
    pub track_count: i64,
    pub total_duration: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    pub artists: Vec<ArtistSummary>,
    pub albums: Vec<AlbumSummary>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumSort {
    Title,
    Artist,
    Release,
    Label,
    Catalog,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtistSort {
    Name,
    Albums,
    Tracks,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackSort {
    Title,
    Artist,
    Album,
    Duration,
    TrackNumber,
}

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Metadata extracted by the scanner from an audio file.
pub struct ScannedTrack {
    pub title: String,
    pub title_sortable: String,
    pub album_id: Option<i64>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration: i64,
    pub location: String,
    pub genres: Option<String>,
    pub artist_names: Option<String>,
    pub folder: Option<String>,
}

pub struct ScannedAlbum {
    pub title: String,
    pub title_sortable: String,
    pub artist_id: i64,
    pub image: Option<Vec<u8>>,
    pub thumb: Option<Vec<u8>>,
    pub release_date: Option<String>,
    pub date_precision: Option<i32>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub isrc: Option<String>,
    pub mbid: String,
    pub vinyl_numbering: bool,
}
