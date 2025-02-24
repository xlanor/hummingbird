use std::{collections::VecDeque, rc::Rc, sync::Arc};

use ahash::AHashMap;
use gpui::*;
use prelude::FluentBuilder;
use tracing::{debug, error};

use crate::{
    library::{
        db::{AlbumMethod, AlbumSortMethod, LibraryAccess},
        scan::ScanEvent,
        types::Album,
    },
    ui::{
        components::table::Table,
        models::Models,
        theme::Theme,
        util::{create_or_retrieve_view, drop_image_from_app, prune_views},
    },
};

use super::ViewSwitchMessage;

#[derive(Clone)]
pub struct AlbumView {
    views_model: Entity<AHashMap<usize, Entity<AlbumItem>>>,
    render_counter: Entity<usize>,
    list_state: ListState,
    view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    table: Entity<Table<Album>>,
}

impl AlbumView {
    pub(super) fn new(
        cx: &mut App,
        view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let album_ids = cx.list_albums(AlbumSortMethod::TitleAsc).map_err(|e| {
                error!("Failed to retrieve album IDs from SQLite: {:?}", e);
            });
            let views_model = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);

            let list_state = AlbumView::make_list_state(
                album_ids.clone().ok(),
                views_model.clone(),
                render_counter.clone(),
                view_switch_model.clone(),
            );

            let state = cx.global::<Models>().scan_state.clone();

            cx.observe(&state, move |this: &mut AlbumView, e, cx| {
                let value = e.read(cx);
                match value {
                    ScanEvent::ScanCompleteIdle => {
                        this.regenerate_list_state(cx);
                    }
                    ScanEvent::ScanProgress { current, .. } => {
                        if current % 50 == 0 {
                            this.regenerate_list_state(cx);
                        }
                    }
                    _ => {}
                }
            })
            .detach();

            let table = Table::new(cx);

            AlbumView {
                views_model,
                render_counter,
                list_state,
                view_switch_model,
                table,
            }
        })
    }

    fn regenerate_list_state<V: 'static>(&mut self, cx: &mut Context<V>) {
        let curr_scroll = self.list_state.logical_scroll_top();
        let album_ids = cx.list_albums(AlbumSortMethod::TitleAsc).map_err(|e| {
            error!("Failed to retrieve album IDs from SQLite: {:?}", e);
        });
        self.views_model = cx.new(|_| AHashMap::new());
        self.render_counter = cx.new(|_| 0);

        self.list_state = AlbumView::make_list_state(
            album_ids.ok(),
            self.views_model.clone(),
            self.render_counter.clone(),
            self.view_switch_model.clone(),
        );

        self.list_state.scroll_to(curr_scroll);

        cx.notify();
    }

    fn make_list_state(
        album_ids: Option<Vec<(u32, String)>>,
        views_model: Entity<AHashMap<usize, Entity<AlbumItem>>>,
        render_counter: Entity<usize>,
        view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    ) -> ListState {
        match album_ids {
            Some(album_ids) => {
                let album_ids_copy = Rc::new(album_ids.clone());

                ListState::new(
                    album_ids.len(),
                    ListAlignment::Top,
                    px(300.0),
                    move |idx, _, cx| {
                        let album_ids = album_ids_copy.clone();
                        let view_switch_model = view_switch_model.clone();

                        prune_views(views_model.clone(), render_counter.clone(), idx, cx);
                        // TODO: error handling
                        div()
                            .w_full()
                            .child(create_or_retrieve_view(
                                views_model.clone(),
                                idx,
                                move |cx| {
                                    AlbumItem::new(cx, album_ids[idx].0 as i64, view_switch_model)
                                },
                                cx,
                            ))
                            .into_any_element()
                    },
                )
            }
            None => ListState::new(0, ListAlignment::Top, px(64.0), move |_, _, _| {
                div().into_any_element()
            }),
        }
    }
}

impl Render for AlbumView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .max_w(px(1000.0))
            .mx_auto()
            .pt(px(24.0))
            .pb(px(0.0))
            .child(self.table.clone())
        // .child(
        //     div()
        //         .w_full()
        //         .pb(px(11.0))
        //         .px(px(24.0))
        //         .line_height(px(26.0))
        //         .font_weight(FontWeight::BOLD)
        //         .text_size(px(26.0))
        //         .child("Albums"),
        // )
        // .child(
        //     div()
        //         .flex()
        //         .w_full()
        //         .border_color(theme.border_color)
        //         .border_b_1()
        //         .child(div().w(px(22.0 + 23.0 + 6.0)).flex_shrink_0())
        //         .child(
        //             div()
        //                 .w(px(294.0))
        //                 .pt(px(6.0))
        //                 .px(px(6.0))
        //                 .pb(px(7.0))
        //                 .w(px(300.0))
        //                 .min_w(px(300.0))
        //                 .max_w(px(300.0))
        //                 .flex_shrink()
        //                 .text_sm()
        //                 .font_weight(FontWeight::BOLD)
        //                 .child("Title"),
        //         )
        //         .child(
        //             div()
        //                 .pt(px(6.0))
        //                 .px(px(6.0))
        //                 .pb(px(7.0))
        //                 .text_sm()
        //                 .font_weight(FontWeight::BOLD)
        //                 .child("Artist"),
        //         ),
        // )
        // .child(list(self.list_state.clone()).w_full().h_full())
    }
}

pub struct AlbumItem {
    album: Option<Arc<Album>>,
    artist: Option<Arc<String>>,
    view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    id: SharedString,
}

impl AlbumItem {
    pub(self) fn new(
        cx: &mut App,
        album_id: i64,
        view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    ) -> Entity<Self> {
        debug!("Creating AlbumItem view for album ID: {}", album_id);

        let album = cx.get_album_by_id(album_id, AlbumMethod::Thumbnail).ok();

        let artist = album
            .as_ref()
            .and_then(|album| cx.get_artist_name_by_id(album.artist_id).ok());

        cx.new(|cx| {
            cx.on_release(|this: &mut Self, cx: &mut App| {
                if let Some(album) = this.album.clone() {
                    if let Some(image) = album.thumb.clone() {
                        drop_image_from_app(cx, image.0);
                        this.album = None;
                        cx.refresh_windows();
                    }
                }
            })
            .detach();

            AlbumItem {
                id: SharedString::from(format!(
                    "album-item-{}",
                    album.as_ref().map(|album| album.id).unwrap_or_default()
                )),
                album,
                artist,
                view_switch_model,
            }
        })
    }
}

impl Render for AlbumItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if let Some(album) = &self.album {
            div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .cursor_pointer()
                .border_b_1()
                .border_color(theme.border_color)
                .px(px(24.0))
                .hover(|this| this.bg(theme.nav_button_hover))
                .active(|this| this.bg(theme.nav_button_active))
                .child(
                    div()
                        .id("album-art")
                        .rounded(px(2.0))
                        .bg(theme.album_art_background)
                        .shadow_sm()
                        .w(px(22.0))
                        .h(px(22.0))
                        .my(px(8.0))
                        .flex_shrink_0()
                        .when(album.thumb.is_some(), |div| {
                            div.child(
                                img(album.thumb.clone().unwrap().0)
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
                        .child(album.title.clone()),
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
                .on_click(cx.listener(|this, _, _, cx| {
                    this.view_switch_model.update(cx, |_, cx| {
                        cx.emit(ViewSwitchMessage::Release(this.album.as_ref().unwrap().id))
                    })
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
