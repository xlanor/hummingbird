use std::{path::Path, sync::Arc, time::Duration};

use async_std::task;
use gpui::{AppContext, Global, WindowContext};
use moka::future::Cache;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tracing::debug;

use crate::ui::app::Pool;

use super::types::{Album, Artist, Track};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlbumMethod {
    Cached,
    Uncached,
    UncachedThumb,
}

pub async fn create_pool(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    debug!("Creating database pool at {:?}", path.as_ref());
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

pub struct DbCache {
    artist_name_cache: Cache<i64, Arc<String>>,
    album_cache: Cache<i64, Arc<Album>>,
    artist_cache: Cache<i64, Arc<Artist>>,
}

impl Global for DbCache {}

pub fn create_cache() -> DbCache {
    let artist_name_cache = Cache::builder()
        .time_to_live(Duration::from_secs(60 * 5))
        .max_capacity(256)
        .build();
    let album_cache = Cache::builder()
        .time_to_live(Duration::from_secs(60 * 5))
        .max_capacity(256)
        .build();
    let artist_cache = Cache::builder()
        .time_to_live(Duration::from_secs(60 * 5))
        .max_capacity(24)
        .build();

    DbCache {
        artist_name_cache,
        album_cache,
        artist_cache,
    }
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
) -> Result<Arc<Vec<Track>>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_tracks_in_album.sql");

    let albums = Arc::new(
        sqlx::query_as::<_, Track>(query)
            .bind(album_id)
            .fetch_all(pool)
            .await?,
    );

    Ok(albums)
}

pub async fn get_album_by_id(
    pool: &SqlitePool,
    db_cache: &DbCache,
    album_id: i64,
    method: AlbumMethod,
) -> Result<Arc<Album>, sqlx::Error> {
    // TODO: this sucks, when if-let chaining comes out fix this
    if let (Some(name), AlbumMethod::Cached) = (db_cache.album_cache.get(&album_id).await, method) {
        Ok(name)
    } else {
        let query = include_str!("../../queries/library/find_album_by_id.sql");

        let album: Arc<Album> = Arc::new({
            let mut data: Album = sqlx::query_as(query).bind(album_id).fetch_one(pool).await?;

            match method {
                AlbumMethod::Cached | AlbumMethod::Uncached => {
                    data.thumb = None;
                }
                AlbumMethod::UncachedThumb => {
                    data.image = None;
                }
            }

            data
        });

        if method == AlbumMethod::Cached {
            db_cache.album_cache.insert(album_id, album.clone()).await;
        }

        Ok(album)
    }
}

pub async fn get_artist_name_by_id(
    pool: &SqlitePool,
    db_cache: &DbCache,
    artist_id: i64,
) -> Result<Arc<String>, sqlx::Error> {
    if let Some(name) = db_cache.artist_name_cache.get(&artist_id).await {
        Ok(name)
    } else {
        let query = include_str!("../../queries/library/find_artist_name_by_id.sql");

        let artist_name: Arc<String> = Arc::new(
            sqlx::query_scalar(query)
                .bind(artist_id)
                .fetch_one(pool)
                .await?,
        );

        db_cache
            .artist_name_cache
            .insert(artist_id, artist_name.clone())
            .await;

        Ok(artist_name)
    }
}

pub async fn get_artist_by_id(
    pool: &SqlitePool,
    db_cache: &DbCache,
    artist_id: i64,
) -> Result<Arc<Artist>, sqlx::Error> {
    if let Some(artist) = db_cache.artist_cache.get(&artist_id).await {
        Ok(artist)
    } else {
        let query = include_str!("../../queries/library/find_artist_by_id.sql");

        let artist: Arc<Artist> = Arc::new(
            sqlx::query_as(query)
                .bind(artist_id)
                .fetch_one(pool)
                .await?,
        );

        db_cache
            .artist_cache
            .insert(artist_id, artist.clone())
            .await;

        Ok(artist)
    }
}

pub trait LibraryAccess {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error>;
    fn list_tracks_in_album(&self, album_id: i64) -> Result<Arc<Vec<Track>>, sqlx::Error>;
    fn get_album_by_id(
        &self,
        album_id: i64,
        method: AlbumMethod,
    ) -> Result<Arc<Album>, sqlx::Error>;
    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<Arc<String>, sqlx::Error>;
    fn get_artist_by_id(&self, artist_id: i64) -> Result<Arc<Artist>, sqlx::Error>;
}

// TODO: profile this with a large library
impl LibraryAccess for AppContext {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_albums(&pool.0, sort_method))
    }

    fn list_tracks_in_album(&self, album_id: i64) -> Result<Arc<Vec<Track>>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_tracks_in_album(&pool.0, album_id))
    }

    fn get_album_by_id(
        &self,
        album_id: i64,
        method: AlbumMethod,
    ) -> Result<Arc<Album>, sqlx::Error> {
        let pool: &Pool = self.global();
        let db_cache: &DbCache = self.global();
        task::block_on(get_album_by_id(&pool.0, db_cache, album_id, method))
    }

    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<Arc<String>, sqlx::Error> {
        let pool: &Pool = self.global();
        let db_cache: &DbCache = self.global();
        task::block_on(get_artist_name_by_id(&pool.0, db_cache, artist_id))
    }

    fn get_artist_by_id(&self, artist_id: i64) -> Result<Arc<Artist>, sqlx::Error> {
        let pool: &Pool = self.global();
        let db_cache: &DbCache = self.global();
        task::block_on(get_artist_by_id(&pool.0, db_cache, artist_id))
    }
}
