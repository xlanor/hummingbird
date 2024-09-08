use std::{rc::Rc, sync::Arc};

use gpui::*;
use prelude::FluentBuilder;
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
            .child(list(self.list_state.clone()).w_full().h_full())
    }
}

pub struct AlbumItem {
    album: Arc<Album>,
    artist: Option<Arc<String>>,
}

impl AlbumItem {
    pub fn new(cx: &mut WindowContext, album_id: i64) -> View<Self> {
        let album = cx
            .get_album_by_id(album_id)
            .expect("Failed to retrieve album");

        let artist = cx.get_artist_name_by_id(album.artist_id).ok();
        cx.new_view(|_| AlbumItem { album, artist })
    }
}

impl Render for AlbumItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .flex()
            .border_b_1()
            .border_color(rgb(0x334155))
            .child(
                div()
                    .pt(px(6.0))
                    .px(px(12.0))
                    .pb(px(7.0))
                    .w(px(200.0))
                    .text_sm()
                    .whitespace_nowrap()
                    .overflow_x_hidden()
                    .when_some(self.artist.clone(), |this, v| this.child((*v).clone())),
            )
            .child(
                div()
                    .pt(px(6.0))
                    .px(px(12.0))
                    .pb(px(7.0))
                    .text_sm()
                    .whitespace_nowrap()
                    .overflow_x_hidden()
                    .w_full()
                    .child(self.album.title.clone()),
            )
    }
}
