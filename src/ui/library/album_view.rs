use std::{rc::Rc, sync::Arc};

use ahash::AHashMap;
use gpui::*;
use prelude::FluentBuilder;
use tracing::{debug, error};

use crate::{
    library::{
        db::{AlbumSortMethod, LibraryAccess},
        types::{Album, Artist},
    },
    ui::util::{create_or_retrieve_view, prune_views},
};

#[derive(Clone)]
pub struct AlbumView {
    album_ids: Model<Vec<(u32, String)>>,
    views_model: Model<AHashMap<usize, View<AlbumItem>>>,
    render_counter: Model<usize>,
    list_state: ListState,
}

impl AlbumView {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            // TODO: update when albums are added or removed
            // this involves clearing views_model and render_counter
            let album_ids = cx.list_albums(AlbumSortMethod::TitleAsc);
            let views_model = cx.new_model(|_| AHashMap::new());
            let render_counter = cx.new_model(|_| 0);
            match album_ids {
                Ok(album_ids) => {
                    let album_ids_copy = Rc::new(album_ids.clone());
                    let views_model_copy = views_model.clone();
                    let render_counter_copy = render_counter.clone();
                    AlbumView {
                        list_state: ListState::new(
                            album_ids.len(),
                            ListAlignment::Top,
                            px(300.0),
                            move |idx, cx| {
                                let album_ids = album_ids_copy.clone();

                                prune_views(
                                    views_model_copy.clone(),
                                    render_counter_copy.clone(),
                                    idx,
                                    cx,
                                );
                                // TODO: error handling
                                div()
                                    .w_full()
                                    .child(create_or_retrieve_view(
                                        views_model_copy.clone(),
                                        idx,
                                        move |cx| AlbumItem::new(cx, album_ids[idx].0 as i64),
                                        cx,
                                    ))
                                    .into_any_element()
                            },
                        ),
                        views_model,
                        render_counter,
                        album_ids: cx.new_model(|_| album_ids),
                    }
                }
                Err(e) => {
                    error!("Failed to retrieve album IDs from SQLite: {:?}", e);
                    error!("Returning empty album view");
                    AlbumView {
                        album_ids: cx.new_model(|_| Vec::new()),
                        views_model,
                        render_counter,
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
        debug!("Creating AlbumItem view for album ID: {}", album_id);

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
