use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use gpui::{App, AppContext, Entity, RenderImage, SharedString};

use crate::{data::types::UIQueueItem, library::db::LibraryAccess, ui::models::Models};

#[derive(Clone, Debug, PartialEq)]
pub struct QueueItemData {
    data: Entity<Option<QueueItemUIData>>,
    db_id: Option<i64>,
    db_album_id: Option<i64>,
    path: String,
}

impl Display for QueueItemData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.path)
    }
}

pub struct QueueItemUIData {
    pub image: Option<Arc<RenderImage>>,
    pub name: Option<SharedString>,
    pub artist_name: Option<SharedString>,
}

impl QueueItemData {
    pub fn new(cx: &mut App, path: String, db_id: Option<i64>, db_album_id: Option<i64>) -> Self {
        QueueItemData {
            path,
            db_id,
            db_album_id,
            data: cx.new(|_| None),
        }
    }

    pub fn get_data(&self, cx: &mut App) -> Entity<Option<QueueItemUIData>> {
        let model = self.data.clone();
        let track_id = self.db_id;
        let album_id = self.db_album_id;
        model.update(cx, move |m, cx| {
            if m.is_none() {
                *m = Some(QueueItemUIData {
                    image: None,
                    name: None,
                    artist_name: None,
                });

                if let (Some(track_id), Some(album_id)) = (track_id, album_id) {
                    let album = cx
                        .get_album_by_id(album_id, crate::library::db::AlbumMethod::UncachedThumb);
                    let track = cx.get_track_by_id(track_id);

                    if let (Ok(track), Ok(album)) = (track, album) {
                        m.as_mut().unwrap().name = Some(track.title.clone().into());
                        m.as_mut().unwrap().image = album.thumb.clone().map(|v| v.0);

                        if let Ok(artist) = cx.get_artist_by_id(album.artist_id) {
                            m.as_mut().unwrap().artist_name = artist.name.clone().map(|v| v.into());
                        }
                    }

                    cx.notify();
                }

                if m.as_ref().unwrap().artist_name.is_none() {
                    // vital information left blank, try retriving the metadata from disk
                    // much slower, especially on windows
                    let queue_model = cx.global::<Models>().queue.clone();

                    cx.subscribe(&queue_model, |m, _, ev: &UIQueueItem, cx| {
                        m.as_mut().unwrap().artist_name = Some(ev.artist_name.clone());
                        m.as_mut().unwrap().image = ev.album_art.clone();
                        m.as_mut().unwrap().name = Some(ev.track_name.clone());
                        cx.notify();
                    })
                    .detach();
                }
            }
        });

        model
    }

    pub fn drop_data(&self, cx: &mut App) {
        self.data.update(cx, |m, cx| {
            *m = None;
            cx.notify();
        });
    }

    pub fn get_path(&self) -> &String {
        &self.path
    }
}
