use std::path::Path;

use async_std::task;
use gpui::WindowContext;
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
) -> Result<Vec<Album>, sqlx::Error> {
    let query = match sort_method {
        AlbumSortMethod::TitleAsc => {
            include_str!("../../queries/library/find_albums_title_asc.sql")
        }
        AlbumSortMethod::TitleDesc => {
            include_str!("../../queries/library/find_albums_title_desc.sql")
        }
    };

    let albums = sqlx::query_as::<_, Album>(query).fetch_all(pool).await?;

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

pub trait LibraryAccess {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<Album>, sqlx::Error>;
    fn list_tracks_in_album(&self, album_id: i64) -> Result<Vec<Track>, sqlx::Error>;
}

// TODO: in theory, blocking DB accesses are not a concern with SQLite read speeds
// but this is only in theory, this needs to be tested with large library sizes
// I'm also worried about loading images upfront here: this seems like a good candidate for a
// virtual scrolling implementation, but not sure how that would be done
impl<'a> LibraryAccess for WindowContext<'a> {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<Album>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_albums(&pool.0, sort_method))
    }

    fn list_tracks_in_album(&self, album_id: i64) -> Result<Vec<Track>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_tracks_in_album(&pool.0, album_id))
    }
}
