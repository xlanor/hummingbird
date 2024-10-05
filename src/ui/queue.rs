use std::path::MAIN_SEPARATOR;

use ahash::AHashMap;
use gpui::*;
use prelude::FluentBuilder;
use tracing::{debug, info};

use crate::{
    data::{interface::GPUIDataInterface, types::UIQueueItem},
    playback::interface::GPUIPlaybackInterface,
};

use super::{
    models::{Models, PlaybackInfo},
    util::{create_or_retrieve_view, prune_views},
};

pub struct QueueItem {
    item: Option<UIQueueItem>,
    path: String,
    current_track: Model<Option<String>>,
    idx: usize,
}

impl QueueItem {
    pub fn new(cx: &mut WindowContext, path: String, idx: usize, clear_cache: bool) -> View<Self> {
        cx.new_view(move |cx| {
            let current_track = cx.global::<PlaybackInfo>().current_track.clone();

            let interface = cx.global::<GPUIDataInterface>();

            if clear_cache {
                interface.evict_cache();
            }

            interface.get_metadata(path.clone());

            let queue_model = cx.global::<Models>().queue.clone();

            cx.subscribe(&queue_model, move |this: &mut QueueItem, _, ev, cx| {
                if ev.file_path == this.path {
                    this.item = Some(ev.clone());
                    cx.notify();
                }
            })
            .detach();

            Self {
                item: None,
                path,
                current_track,
                idx,
            }
        })
    }
}

impl Render for QueueItem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        if let Some(item) = self.item.as_ref() {
            let is_current = self
                .current_track
                .read(cx)
                .as_ref()
                .map(|v| v == &item.file_path)
                .unwrap_or(false);

            let album_art = item
                .album_art
                .as_ref()
                .map(|v| ImageSource::Render(v.clone()));

            let idx = self.idx;

            div()
                .w_full()
                .id(ElementId::View(cx.entity_id()))
                .flex()
                .overflow_x_hidden()
                .gap(px(11.0))
                .p(px(11.0))
                .border_t(px(1.0))
                .border_color(rgb(0x1e293b))
                .when(is_current, |div| div.bg(rgb(0x1f2937)))
                .on_click(move |_, cx| {
                    cx.global::<GPUIPlaybackInterface>().jump(idx);
                })
                .hover(|div| div.bg(rgb(0x1f2937)))
                .active(|div| div.bg(rgb(0x030712)))
                .child(
                    div()
                        .id("album-art")
                        .rounded(px(4.0))
                        .bg(rgb(0x4b5563))
                        .shadow_sm()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex_shrink_0()
                        .when(album_art.is_some(), |div| {
                            div.child(
                                img(album_art.unwrap())
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded(px(4.0)),
                            )
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .line_height(rems(1.0))
                        .text_size(px(15.0))
                        .gap_1()
                        .overflow_x_hidden()
                        .child(
                            div()
                                .text_ellipsis()
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .child(
                                    item.metadata
                                        .artist
                                        .clone()
                                        .unwrap_or("Unknown Artist".into()),
                                ),
                        )
                        .child(
                            div()
                                .text_ellipsis()
                                .child(item.metadata.name.clone().unwrap_or(
                                    item.file_path.split(MAIN_SEPARATOR).last().unwrap().into(),
                                )),
                        ),
                )
        } else {
            // TODO: Skeleton for this
            div().id(ElementId::View(cx.entity_id()))
        }
    }
}

pub struct Queue {
    views_model: Model<AHashMap<usize, View<QueueItem>>>,
    render_counter: Model<usize>,
    state: ListState,
}

impl Queue {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let views_model = cx.new_model(|_| AHashMap::new());
            let render_counter = cx.new_model(|_| 0);
            let items = cx.global::<Models>().queue.clone();

            cx.observe(&items, move |this: &mut Queue, m, cx| {
                this.views_model = cx.new_model(|_| AHashMap::new());
                this.render_counter = cx.new_model(|_| 0);

                let items = m.read(cx).clone();
                let views_model = this.views_model.clone();
                let render_counter = this.render_counter.clone();

                this.state = ListState::new(
                    items.0.len(),
                    ListAlignment::Top,
                    px(200.0),
                    move |idx, cx| {
                        let item = items.0.get(idx).unwrap().clone();
                        let was_removed =
                            prune_views(views_model.clone(), render_counter.clone(), idx, cx);

                        div()
                            .child(create_or_retrieve_view(
                                views_model.clone(),
                                idx,
                                move |cx| QueueItem::new(cx, item, idx, was_removed),
                                cx,
                            ))
                            .into_any_element()
                    },
                );

                cx.notify();
            })
            .detach();

            Self {
                views_model,
                render_counter,
                state: ListState::new(0, ListAlignment::Top, px(200.0), move |_, _| {
                    div().into_any_element()
                }),
            }
        })
    }
}

impl Render for Queue {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            // .absolute()
            // .top_0()
            // .right_0()
            .h_full()
            .min_w(px(275.0))
            .max_w(px(275.0))
            .w(px(275.0))
            .border_l(px(1.0))
            .flex_shrink_0()
            .border_color(rgb(0x1e293b))
            .pb(px(0.0))
            .flex()
            .flex_col()
            .child(
                div().flex().border_b_1().border_color(rgb(0x1e293b)).child(
                    div()
                        .flex()
                        .w_full()
                        .child(
                            div()
                                .ml(px(12.0))
                                .pt(px(7.0))
                                .flex()
                                .font_weight(FontWeight::BOLD)
                                .child(div().text_sm().child("Queue")),
                        )
                        .child(
                            div()
                                .flex()
                                .id("back")
                                .font_family("Font Awesome 6 Free")
                                .pr(px(16.0))
                                .pl(px(16.0))
                                .py(px(8.0))
                                .ml_auto()
                                .text_sm()
                                .border_l_1()
                                .border_color(rgb(0x1e293b))
                                .hover(|this| this.bg(rgb(0x1e293b)))
                                .active(|this| this.bg(rgb(0x111827)))
                                .cursor_pointer()
                                .child(""),
                        ),
                ),
            )
            .child(
                div()
                    .w_full()
                    .pt(px(24.0))
                    .pb(px(11.0))
                    .px(px(12.0))
                    .line_height(px(26.0))
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(26.0))
                    .child("Queue"),
            )
            .child(list(self.state.clone()).w_full().h_full().flex().flex_col())
    }
}
