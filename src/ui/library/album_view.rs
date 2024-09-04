use std::rc::Rc;

use gpui::*;
use tracing::{debug, error};

use crate::library::{
    db::{AlbumSortMethod, LibraryAccess},
    types::Album,
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
                                let album = Rc::new(
                                    cx.get_album_by_id(album_ids_copy[idx].0 as i64)
                                        .expect("Failed to retrieve album"),
                                );
                                div()
                                    .w(px(100.0))
                                    .h(px(200.0))
                                    .bg(rgb(0x00FF00))
                                    .child(AlbumItem::new(cx, album.clone()))
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
}

impl AlbumItem {
    pub fn new(cx: &mut WindowContext, album: Rc<Album>) -> View<Self> {
        cx.new_view(|_| AlbumItem { album })
    }
}

impl Render for AlbumItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        debug!("Rendering album item: {:?}", self.album.title.clone());
        div()
            .w(px(100.0))
            .h(px(200.0))
            .bg(rgb(0x0000FF))
            .child("album")
            .child(format!(
                "{} - {}",
                self.album.artist_id,
                self.album.title.clone()
            ))
    }
}

// impl RenderOnce for AlbumItem {
//     fn render(self, cx: &mut WindowContext) -> impl IntoElement {
//         debug!("Rendering album item: {:?}", self.album.title.clone());
//         div()
//             .w(px(100.0))
//             .h(px(200.0))
//             .bg(rgb(0x0000FF))
//             .child("album")
//             .child(format!(
//                 "{} - {}",
//                 self.album.artist_id,
//                 self.album.title.clone()
//             ))
//     }
// }
