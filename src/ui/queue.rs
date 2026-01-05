use crate::{
    playback::{
        interface::PlaybackInterface,
        queue::{DataSource, QueueItemData},
    },
    settings::storage::DEFAULT_QUEUE_WIDTH,
    ui::components::{
        context::context,
        drag_drop::{
            DragData, DragDropItemState, DragDropListConfig, DragDropListManager, DragPreview,
            DropIndicator, check_drag_cancelled, continue_edge_scroll, handle_drag_move,
            handle_drop,
        },
        icons::{CROSS, SHUFFLE, TRASH, icon},
        menu::{menu, menu_item},
        nav_button::nav_button,
        resizable_sidebar::{ResizeSide, resizable_sidebar},
        scrollbar::{RightPad, ScrollableHandle, floating_scrollbar},
    },
};
use gpui::*;
use prelude::FluentBuilder;
use rustc_hash::FxHashMap;

use super::{
    components::button::{ButtonSize, ButtonStyle, button},
    models::{Models, PlaybackInfo},
    theme::Theme,
    util::{create_or_retrieve_view, drop_image_from_app, prune_views},
};

/// The list identifier for queue drag-drop operations
const QUEUE_LIST_ID: &str = "queue";
/// Height of each queue item in pixels
const QUEUE_ITEM_HEIGHT: f32 = 59.0;

pub struct QueueItem {
    item: Option<QueueItemData>,
    current: usize,
    idx: usize,
    drag_drop_manager: Entity<DragDropListManager>,
}

impl QueueItem {
    pub fn new(
        cx: &mut App,
        item: Option<QueueItemData>,
        idx: usize,
        drag_drop_manager: Entity<DragDropListManager>,
    ) -> Entity<Self> {
        cx.new(move |cx| {
            cx.on_release(|m: &mut QueueItem, cx| {
                if let Some(item) = m.item.as_mut() {
                    let data = item.get_data(cx).read(cx).as_ref().unwrap();

                    if let (Some(image), DataSource::Library) = (data.image.clone(), data.source) {
                        drop_image_from_app(cx, image);
                    }

                    item.drop_data(cx);
                }
            })
            .detach();

            let queue = cx.global::<Models>().queue.clone();

            cx.observe(&queue, |this: &mut QueueItem, queue, cx| {
                this.current = queue.read(cx).position;
            })
            .detach();

            let data = item.as_ref().unwrap().get_data(cx);

            cx.observe(&data, |_, _, cx| {
                cx.notify();
            })
            .detach();

            // Observe drag-drop state changes to update visual feedback
            cx.observe(&drag_drop_manager, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                item,
                idx,
                current: queue.read(cx).position,
                drag_drop_manager,
            }
        })
    }
}

impl Render for QueueItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let data = self
            .item
            .as_ref()
            .and_then(|item| item.get_data(cx).read(cx).clone());
        let theme = cx.global::<Theme>().clone();

        if let Some(item) = data.as_ref() {
            let is_current = self.current == self.idx;
            let album_art = item.image.as_ref().cloned();
            let idx = self.idx;

            let item_state =
                DragDropItemState::for_index(&self.drag_drop_manager.read(cx), self.idx);

            let track_name = item.name.clone().unwrap_or_else(|| "Unknown Track".into());

            context(ElementId::View(cx.entity_id()))
                .with(
                    div()
                        .w_full()
                        .id("item-contents")
                        .flex()
                        .flex_shrink_0()
                        .overflow_x_hidden()
                        .gap(px(11.0))
                        .h(px(QUEUE_ITEM_HEIGHT))
                        .p(px(11.0))
                        .cursor_pointer()
                        .relative()
                        // Default bottom border - always present
                        .border_b(px(1.0))
                        .border_color(theme.border_color)
                        .when(item_state.is_being_dragged, |div| div.opacity(0.5))
                        .when(is_current && !item_state.is_being_dragged, |div| {
                            div.bg(theme.queue_item_current)
                        })
                        .on_click(move |_, _, cx| {
                            cx.global::<PlaybackInterface>().jump(idx);
                        })
                        .when(!item_state.is_being_dragged, |div| {
                            div.hover(|div| div.bg(theme.queue_item_hover))
                                .active(|div| div.bg(theme.queue_item_active))
                        })
                        .on_drag(DragData::new(idx, QUEUE_LIST_ID), move |_, _, _, cx| {
                            DragPreview::new(cx, track_name.clone())
                        })
                        .drag_over::<DragData>(move |style, _, _, _| {
                            style.bg(gpui::rgba(0x88888822))
                        })
                        .child(DropIndicator::with_state(
                            item_state.is_drop_target_before,
                            item_state.is_drop_target_after,
                            theme.button_primary,
                        ))
                        .child(
                            div()
                                .id("album-art")
                                .rounded(px(4.0))
                                .bg(theme.album_art_background)
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
                                        .when_some(item.name.clone(), |this, string| {
                                            this.child(string)
                                        }),
                                )
                                .child(
                                    div()
                                        .text_ellipsis()
                                        .when_some(item.artist_name.clone(), |this, string| {
                                            this.child(string)
                                        }),
                                ),
                        ),
                )
                .child(menu().item(menu_item(
                    "remove-item",
                    Some(CROSS),
                    "Remove from queue",
                    move |_, _, cx| {
                        let playback = cx.global::<PlaybackInterface>();
                        playback.remove_item(idx);
                    },
                )))
                .into_any_element()
        } else {
            // TODO: Skeleton for this
            div()
                .h(px(QUEUE_ITEM_HEIGHT))
                .border_t(px(1.0))
                .border_color(theme.border_color)
                .w_full()
                .id(ElementId::View(cx.entity_id()))
                .into_any_element()
        }
    }
}

pub struct Queue {
    views_model: Entity<FxHashMap<usize, Entity<QueueItem>>>,
    render_counter: Entity<usize>,
    shuffling: Entity<bool>,
    show_queue: Entity<bool>,
    scroll_handle: UniformListScrollHandle,
    drag_drop_manager: Entity<DragDropListManager>,
}

impl Queue {
    pub fn new(cx: &mut App, show_queue: Entity<bool>) -> Entity<Self> {
        cx.new(|cx| {
            let views_model = cx.new(|_| FxHashMap::default());
            let render_counter = cx.new(|_| 0);
            let items = cx.global::<Models>().queue.clone();

            let config = DragDropListConfig::new(QUEUE_LIST_ID, px(QUEUE_ITEM_HEIGHT));
            let drag_drop_manager = DragDropListManager::new(cx, config);

            cx.observe(&items, move |this: &mut Queue, _, cx| {
                this.views_model = cx.new(|_| FxHashMap::default());
                this.render_counter = cx.new(|_| 0);

                cx.notify();
            })
            .detach();

            let shuffling = cx.global::<PlaybackInfo>().shuffling.clone();

            cx.observe(&shuffling, |_, _, cx| {
                cx.notify();
            })
            .detach();

            let queue_width = cx.global::<Models>().queue_width.clone();
            cx.observe(&queue_width, |_, _, cx| cx.notify()).detach();

            Self {
                views_model,
                render_counter,
                shuffling,
                show_queue,
                scroll_handle: UniformListScrollHandle::new(),
                drag_drop_manager,
            }
        })
    }
}

impl Render for Queue {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        check_drag_cancelled(self.drag_drop_manager.clone(), cx);

        let theme = cx.global::<Theme>();
        let queue = cx
            .global::<Models>()
            .queue
            .clone()
            .read(cx)
            .data
            .read()
            .expect("could not read queue");
        let queue_len = queue.len();
        let shuffling = self.shuffling.read(cx);
        let views_model = self.views_model.clone();
        let render_counter = self.render_counter.clone();
        let scroll_handle = self.scroll_handle.clone();
        let drag_drop_manager = self.drag_drop_manager.clone();

        let queue_width = cx.global::<Models>().queue_width.clone();

        resizable_sidebar("queue-resizable", queue_width.clone(), ResizeSide::Left)
            .min_width(px(225.0))
            .max_width(px(450.0))
            .default_width(DEFAULT_QUEUE_WIDTH)
            .h_full()
            .child(
                div()
                    .h_full()
                    .w_full()
                    .border_l(px(1.0))
                    .border_color(theme.border_color)
                    .pb(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        div().flex().child(
                            div().flex().w_full().child(
                                nav_button("close", CROSS)
                                    .mt(px(9.0))
                                    .mr(px(9.0))
                                    .ml_auto()
                                    .on_click(cx.listener(|this: &mut Self, _, _, cx| {
                                        this.show_queue.update(cx, |v, _| *v = !(*v))
                                    })),
                            ),
                        ),
                    )
                    .child(
                        div()
                            .w_full()
                            .pt(px(9.0))
                            .pb(px(12.0))
                            .px(px(12.0))
                            .flex()
                            .child(
                                div()
                                    .line_height(px(26.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_size(px(26.0))
                                    .child("Queue"),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .border_t_1()
                            .border_b_1()
                            .border_color(theme.border_color)
                            .child(
                                button()
                                    .style(ButtonStyle::MinimalNoRounding)
                                    .size(ButtonSize::Large)
                                    .child(icon(TRASH).size(px(14.0)).my_auto())
                                    .child("Clear")
                                    .w_full()
                                    .id("clear-queue")
                                    .on_click(|_, _, cx| {
                                        cx.global::<PlaybackInterface>().clear_queue();
                                        cx.global::<PlaybackInterface>().stop();
                                    }),
                            )
                            .child(
                                button()
                                    .style(ButtonStyle::MinimalNoRounding)
                                    .size(ButtonSize::Large)
                                    .child(icon(SHUFFLE).size(px(14.0)).my_auto())
                                    .when(*shuffling, |this| this.child("Shuffling"))
                                    .when(!shuffling, |this| this.child("Shuffle"))
                                    .w_full()
                                    .id("queue-shuffle")
                                    .on_click(|_, _, cx| {
                                        cx.global::<PlaybackInterface>().toggle_shuffle()
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("queue-list-container")
                            .flex()
                            .w_full()
                            .h_full()
                            .relative()
                            .on_drag_move::<DragData>(cx.listener(
                                move |this: &mut Queue,
                                      event: &DragMoveEvent<DragData>,
                                      window,
                                      cx| {
                                    let scroll_handle: ScrollableHandle =
                                        this.scroll_handle.clone().into();

                                    let scrolled = handle_drag_move(
                                        this.drag_drop_manager.clone(),
                                        scroll_handle,
                                        event,
                                        queue_len,
                                        cx,
                                    );

                                    if scrolled {
                                        let entity = cx.entity().downgrade();
                                        let manager = this.drag_drop_manager.clone();
                                        let scroll_handle: ScrollableHandle =
                                            this.scroll_handle.clone().into();

                                        window.on_next_frame(move |window, cx| {
                                            if let Some(entity) = entity.upgrade() {
                                                entity.update(cx, |_, cx| {
                                                    Self::schedule_edge_scroll(
                                                        manager,
                                                        scroll_handle,
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }
                                        });
                                    }

                                    cx.notify();
                                },
                            ))
                            .on_drop(cx.listener(
                                move |this: &mut Queue, drag_data: &DragData, _, cx| {
                                    handle_drop(
                                        this.drag_drop_manager.clone(),
                                        drag_data,
                                        cx,
                                        |from, to, cx| {
                                            cx.global::<PlaybackInterface>().move_item(from, to);
                                        },
                                    );
                                    cx.notify();
                                },
                            ))
                            .child(
                                uniform_list("queue", queue_len, move |range, _, cx| {
                                    let start = range.start;
                                    let is_templ_render = range.start == 0 && range.end == 1;

                                    let queue = cx
                                        .global::<Models>()
                                        .queue
                                        .clone()
                                        .read(cx)
                                        .data
                                        .read()
                                        .expect("could not read queue");

                                    if range.end <= queue.len() {
                                        let items = queue[range].to_vec();

                                        drop(queue);

                                        items
                                            .into_iter()
                                            .enumerate()
                                            .map(|(idx, item)| {
                                                let idx = idx + start;

                                                if !is_templ_render {
                                                    prune_views(
                                                        &views_model,
                                                        &render_counter,
                                                        idx,
                                                        cx,
                                                    );
                                                }

                                                let drag_drop_manager = drag_drop_manager.clone();

                                                div().child(create_or_retrieve_view(
                                                    &views_model,
                                                    idx,
                                                    move |cx| {
                                                        QueueItem::new(
                                                            cx,
                                                            Some(item),
                                                            idx,
                                                            drag_drop_manager,
                                                        )
                                                    },
                                                    cx,
                                                ))
                                            })
                                            .collect()
                                    } else {
                                        Vec::new()
                                    }
                                })
                                .w_full()
                                .h_full()
                                .flex()
                                .flex_col()
                                .track_scroll(scroll_handle.clone()),
                            )
                            .child(floating_scrollbar(
                                "queue_scrollbar",
                                scroll_handle,
                                RightPad::Pad,
                            )),
                    ),
            )
    }
}

impl Queue {
    fn schedule_edge_scroll(
        manager: Entity<DragDropListManager>,
        scroll_handle: ScrollableHandle,
        window: &mut Window,
        cx: &mut App,
    ) {
        let should_continue = continue_edge_scroll(&manager.read(cx), &scroll_handle);

        if should_continue {
            let manager_clone = manager.clone();
            let scroll_handle_clone = scroll_handle.clone();

            window.on_next_frame(move |window, cx| {
                Self::schedule_edge_scroll(manager_clone, scroll_handle_clone, window, cx);
            });

            window.refresh();
        }
    }
}
