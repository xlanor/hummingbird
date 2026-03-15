pub mod dao;

use chrono::{DateTime, Utc};
use serde::Serialize;

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
