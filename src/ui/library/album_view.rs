use std::{collections::VecDeque, rc::Rc, sync::Arc};

use ahash::AHashMap;
use gpui::*;
use prelude::FluentBuilder;
use tracing::{debug, error};

use crate::{
    data::{
        events::{ImageLayout, ImageType},
        interface::GPUIDataInterface,
    },
    library::{
        db::{AlbumSortMethod, LibraryAccess},
        types::{Album, Artist},
    },
    ui::{
        models::{Models, TransferDummy},
        util::{create_or_retrieve_view, prune_views},
    },
};

use super::{LibraryView, ViewSwitchDummy, ViewSwitchMessage};

#[derive(Clone)]
pub struct AlbumView {
    album_ids: Model<Vec<(u32, String)>>,
    views_model: Model<AHashMap<usize, View<AlbumItem>>>,
    render_counter: Model<usize>,
    list_state: ListState,
    view_switch_model: Model<VecDeque<ViewSwitchMessage>>,
}

impl AlbumView {
    pub(super) fn new<V: 'static>(
        cx: &mut ViewContext<V>,
        view_switch_model: Model<VecDeque<ViewSwitchMessage>>,
    ) -> View<Self> {
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
                    let view_switch_model_copy = view_switch_model.clone();
                    AlbumView {
                        list_state: ListState::new(
                            album_ids.len(),
                            ListAlignment::Top,
                            px(300.0),
                            move |idx, cx| {
                                let album_ids = album_ids_copy.clone();
                                let view_switch_model = view_switch_model_copy.clone();

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
                                        move |cx| {
                                            AlbumItem::new(
                                                cx,
                                                album_ids[idx].0 as i64,
                                                view_switch_model,
                                            )
                                        },
                                        cx,
                                    ))
                                    .into_any_element()
                            },
                        ),
                        views_model,
                        render_counter,
                        album_ids: cx.new_model(|_| album_ids),
                        view_switch_model,
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
                        view_switch_model,
                    }
                }
            }
        })
    }
}

impl Render for AlbumView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .max_w(px(1000.0))
            .mx_auto()
            .pt(px(24.0))
            .pb(px(0.0))
            .px(px(12.0))
            .child(
                div()
                    .w_full()
                    .mb(px(11.0))
                    .px(px(12.0))
                    .line_height(px(26.0))
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(26.0))
                    .child("Albums"),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .border_color(rgb(0x1e293b))
                    .border_b_1()
                    .child(div().w(px(22.0 + 11.0 + 6.0)).flex_shrink_0())
                    .child(
                        div()
                            .w(px(294.0))
                            .pt(px(6.0))
                            .px(px(6.0))
                            .pb(px(7.0))
                            .w(px(300.0))
                            .min_w(px(300.0))
                            .max_w(px(300.0))
                            .flex_shrink()
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child("Title"),
                    )
                    .child(
                        div()
                            .pt(px(6.0))
                            .px(px(6.0))
                            .pb(px(7.0))
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child("Artist"),
                    ),
            )
            .child(list(self.list_state.clone()).w_full().h_full())
    }
}

pub struct AlbumItem {
    album: Arc<Album>,
    artist: Option<Arc<String>>,
    image_transfer_model: Model<TransferDummy>,
    image: Option<Arc<RenderImage>>,
    view_switch_model: Model<VecDeque<ViewSwitchMessage>>,
    id: SharedString,
}

impl AlbumItem {
    pub(self) fn new(
        cx: &mut WindowContext,
        album_id: i64,
        view_switch_model: Model<VecDeque<ViewSwitchMessage>>,
    ) -> View<Self> {
        debug!("Creating AlbumItem view for album ID: {}", album_id);

        let album = cx
            .get_album_by_id(album_id)
            .expect("Failed to retrieve album");

        let artist = cx.get_artist_name_by_id(album.artist_id).ok();
        cx.new_view(|cx| {
            let model = cx.global::<Models>().image_transfer_model.clone();

            cx.subscribe(&model, move |this: &mut AlbumItem, _, image, cx| {
                if image.0 == ImageType::AlbumArt(album_id) {
                    debug!("captured decoded image for album ID: {}", album_id);
                    this.image = Some(image.1.clone());
                    cx.notify();
                }
            })
            .detach();

            if let Some(image) = album.image.clone() {
                cx.global::<GPUIDataInterface>().decode_image(
                    image,
                    ImageType::AlbumArt(album_id),
                    ImageLayout::BGR,
                    true,
                );
            }

            AlbumItem {
                id: SharedString::from(format!("album-item-{}", album.id)),
                album,
                artist,
                image_transfer_model: model,
                image: None,
                view_switch_model,
            }
        })
    }
}

impl Render for AlbumItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .cursor_pointer()
            .border_b_1()
            .border_color(rgb(0x1e293b))
            .child(
                div()
                    .id("album-art")
                    .rounded(px(2.0))
                    .bg(rgb(0x4b5563))
                    .shadow_sm()
                    .w(px(22.0))
                    .h(px(22.0))
                    .ml(px(12.0))
                    .my(px(8.0))
                    .flex_shrink_0()
                    .when(self.image.is_some(), |div| {
                        div.child(
                            img(self.image.clone().unwrap())
                                .w(px(22.0))
                                .h(px(22.0))
                                .rounded(px(2.0)),
                        )
                    }),
            )
            .child(
                div()
                    .my_auto()
                    .px(px(12.0))
                    .pb(px(1.0))
                    .w(px(300.0))
                    .min_w(px(300.0))
                    .max_w(px(300.0))
                    .flex_shrink()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.album.title.clone()),
            )
            .child(
                div()
                    .my_auto()
                    .pb(px(1.0))
                    .px(px(12.0))
                    .text_sm()
                    .flex_shrink()
                    .whitespace_nowrap()
                    .overflow_x_hidden()
                    .when_some(self.artist.clone(), |this, v| this.child((*v).clone())),
            )
            .on_click(cx.listener(|this, _, cx| {
                this.view_switch_model.update(cx, |_, cx| {
                    cx.emit(ViewSwitchMessage::Release(this.album.id))
                })
            }))
    }
}
