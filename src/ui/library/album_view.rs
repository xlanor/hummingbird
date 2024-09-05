use std::rc::Rc;

use gpui::*;
use tracing::{debug, error};

use crate::library::{
    db::{AlbumSortMethod, LibraryAccess},
    types::{Album, Artist},
};

#[derive(Clone)]
pub struct AlbumView {
    album_ids: Model<Vec<(u32, String)>>,
    list_state: ListState,
}

impl AlbumView {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let album_ids = cx.list_albums(AlbumSortMethod::TitleAsc);
            match album_ids {
                Ok(album_ids) => {
                    debug!("Retrieved {} album IDs from SQLite", album_ids.len());
                    let album_ids_copy = album_ids.clone();
                    AlbumView {
                        list_state: ListState::new(
                            album_ids.len(),
                            ListAlignment::Top,
                            px(300.0),
                            move |idx, cx| {
                                // TODO: error handling
                                div()
                                    .w_full()
                                    .bg(rgb(0x00FF00))
                                    .child(AlbumItem::new(cx, album_ids_copy[idx].0 as i64))
                                    .into_any_element()
                            },
                        ),
                        album_ids: cx.new_model(|_| album_ids),
                    }
                }
                Err(e) => {
                    error!("Failed to retrieve album IDs from SQLite: {:?}", e);
                    error!("Returning empty album view");
                    AlbumView {
                        album_ids: cx.new_model(|_| Vec::new()),
                        list_state: ListState::new(
                            0,
                            ListAlignment::Top,
                            px(300.0),
                            move |_, _| div().into_any_element(),
                        ),
                    }
                }
            }
        })
    }
}

impl Render for AlbumView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .flex()
            .bg(rgb(0xFF0000))
            .child(list(self.list_state.clone()).w_full().h_full())
    }
}

pub struct AlbumItem {
    album: Rc<Album>,
    artist: Rc<Option<String>>,
}

impl AlbumItem {
    pub fn new(cx: &mut WindowContext, album_id: i64) -> View<Self> {
        let album = Rc::new(
            cx.get_album_by_id(album_id as i64)
                .expect("Failed to retrieve album"),
        );
        let artist = Rc::new(cx.get_artist_name_by_id(album.artist_id).ok());
        cx.new_view(|_| AlbumItem { album, artist })
    }
}

impl Render for AlbumItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        debug!("Rendering album item: {:?}", self.album.title.clone());
        div().w_full().child(format!(
            "{} - {}",
            (*self.artist).clone().unwrap_or("Unknown".to_string()),
            self.album.title.clone()
        ))
    }
}
