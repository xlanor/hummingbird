use std::path::Path;

use async_std::task;
use gpui::{AppContext, WindowContext};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tracing::debug;

use crate::ui::app::Pool;

use super::types::{Album, Track};

pub async fn create_pool(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    debug!("Creating database pool at {:?}", path.as_ref());
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlbumSortMethod {
    TitleAsc,
    TitleDesc,
}

pub async fn list_albums(
    pool: &SqlitePool,
    sort_method: AlbumSortMethod,
) -> Result<Vec<(u32, String)>, sqlx::Error> {
    let query = match sort_method {
        AlbumSortMethod::TitleAsc => {
            include_str!("../../queries/library/find_albums_title_asc.sql")
        }
        AlbumSortMethod::TitleDesc => {
            include_str!("../../queries/library/find_albums_title_desc.sql")
        }
    };

    let albums = sqlx::query_as::<_, (u32, String)>(query)
        .fetch_all(pool)
        .await?;

    Ok(albums)
}

pub async fn list_tracks_in_album(
    pool: &SqlitePool,
    album_id: i64,
) -> Result<Vec<Track>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_tracks_in_album.sql");

    let albums = sqlx::query_as::<_, Track>(query)
        .bind(album_id)
        .fetch_all(pool)
        .await?;

    Ok(albums)
}

pub async fn get_album_by_id(pool: &SqlitePool, album_id: i64) -> Result<Album, sqlx::Error> {
    let query = include_str!("../../queries/library/find_album_by_id.sql");

    let album = sqlx::query_as::<_, Album>(query)
        .bind(album_id)
        .fetch_one(pool)
        .await?;

    Ok(album)
}

pub async fn get_artist_name_by_id(
    pool: &SqlitePool,
    artist_id: i64,
) -> Result<String, sqlx::Error> {
    let query = include_str!("../../queries/library/find_artist_name_by_id.sql");

    let artist_name = sqlx::query_scalar(query)
        .bind(artist_id)
        .fetch_one(pool)
        .await?;

    Ok(artist_name)
}

pub trait LibraryAccess {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error>;
    fn list_tracks_in_album(&self, album_id: i64) -> Result<Vec<Track>, sqlx::Error>;
    fn get_album_by_id(&self, album_id: i64) -> Result<Album, sqlx::Error>;
    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<String, sqlx::Error>;
}

// TODO: profile this with a large library
impl LibraryAccess for AppContext {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_albums(&pool.0, sort_method))
    }

    fn list_tracks_in_album(&self, album_id: i64) -> Result<Vec<Track>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_tracks_in_album(&pool.0, album_id))
    }

    fn get_album_by_id(&self, album_id: i64) -> Result<Album, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_album_by_id(&pool.0, album_id))
    }

    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<String, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_artist_name_by_id(&pool.0, artist_id))
    }
}
