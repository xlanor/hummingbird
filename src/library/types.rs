use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::RenderImage;
use image::Frame;
use smallvec::SmallVec;
use sqlx::{Database, Decode, Sqlite, ValueRef};

use crate::util::rgb_to_bgr;

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

#[derive(Clone)]
pub struct Thumbnail(pub Arc<RenderImage>);

impl Thumbnail {
    pub fn new(image: Arc<RenderImage>) -> Self {
        Self(image)
    }
}

impl From<Box<[u8]>> for Thumbnail {
    fn from(data: Box<[u8]>) -> Self {
        let mut image = image::load_from_memory(&data)
            .unwrap()
            .as_rgba8()
            .expect("invalid thumbnail")
            .to_owned();

        rgb_to_bgr(&mut image);

        Self(Arc::new(RenderImage::new(SmallVec::from_vec(vec![
            Frame::new(image),
        ]))))
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Thumbnail
where
    Box<[u8]>: Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let data = <Box<[u8]>>::decode(value)?;
        Ok(Self::from(data))
    }
}

impl<'q, DB: Database> sqlx::Encode<'q, DB> for Thumbnail
where
    Box<[u8]>: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        panic!("Thumbnail is write-only")
    }
}

impl sqlx::Type<sqlx::Sqlite> for Thumbnail {
    fn type_info() -> <Sqlite as Database>::TypeInfo {
        <Box<[u8]>>::type_info()
    }
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
    pub thumb: Option<Thumbnail>,
    #[sqlx(default)]
    pub image_mime: Option<String>,
    #[sqlx(skip)]
    pub tags: Option<Vec<String>>,
    #[sqlx(default)]
    pub label: Option<String>,
    #[sqlx(default)]
    pub catalog_number: Option<String>,
    #[sqlx(default)]
    pub isrc: Option<String>,
}

#[derive(sqlx::FromRow, Clone)]
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
