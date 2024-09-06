use std::path::{Path, PathBuf};

#[derive(sqlx::FromRow)]
pub struct Artist {
    pub id: i64,
    pub name: Option<String>,
    pub name_sortable: Option<String>,
    #[sqlx(default)]
    pub bio: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    #[sqlx(default)]
    pub image: Option<Box<[u8]>>,
    #[sqlx(default)]
    pub image_mime: Option<String>,
    #[sqlx(skip)]
    pub tags: Option<Vec<String>>,
}

#[derive(sqlx::FromRow, Clone)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub title_sortable: String,
    pub artist_id: i64,
    #[sqlx(default)]
    pub release_date: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    #[sqlx(default)]
    pub image: Option<Box<[u8]>>,
    #[sqlx(default)]
    pub image_mime: Option<String>,
    #[sqlx(skip)]
    pub tags: Option<Vec<String>>,
}

#[derive(sqlx::FromRow)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub title_sortable: String,
    #[sqlx(default)]
    pub album_id: Option<i64>,
    #[sqlx(default)]
    pub track_number: Option<i32>,
    #[sqlx(default)]
    pub disc_number: Option<i32>,
    #[sqlx(default)]
    pub duration: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    #[sqlx(skip)]
    pub genres: Option<Vec<String>>,
    #[sqlx(skip)]
    pub tags: Option<Vec<String>>,
    pub location: String,
}
