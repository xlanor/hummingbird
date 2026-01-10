use std::sync::Arc;

use gpui::{
    App, AppContext, Context, DragMoveEvent, Entity, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, UniformListScrollHandle, Window, actions, div, prelude::FluentBuilder, px, rems, rgba,
    uniform_list,
};
use rustc_hash::FxHashMap;
use tracing::error;

use crate::{
    library::{
        db::LibraryAccess,
        playlist::export_playlist,
        types::{Playlist, PlaylistType},
    },
    playback::{
        interface::{PlaybackInterface, replace_queue},
        queue::QueueItemData,
    },
    ui::{
        caching::hummingbird_cache,
        command_palette::{Command, CommandManager},
        components::{
            button::{ButtonIntent, ButtonSize, button},
            drag_drop::{
                DragData, DragDropItemState, DragDropListConfig, DragDropListManager, DragPreview,
                DropIndicator, check_drag_cancelled, continue_edge_scroll, handle_drag_move,
                handle_drop,
            },
            icons::{CIRCLE_PLUS, PLAY, PLAYLIST, SHUFFLE, STAR, icon},
            scrollbar::{RightPad, ScrollableHandle, floating_scrollbar},
        },
        library::track_listing::{
            ArtistNameVisibility,
            track_item::{TrackItem, TrackItemLeftField},
        },
        models::{Models, PlaybackInfo, PlaylistEvent},
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

use super::track_listing::track_item::TrackPlaylistInfo;

actions!(playlist, [Export, Import]);

// height + border
const PLAYLIST_ITEM_HEIGHT: f32 = 40.0;

pub fn bind_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("secondary-s", Export, None)]);
}

/// Wrapper component for playlist track items that adds drag-and-drop support
pub struct PlaylistTrackItem {
    track_item: Entity<TrackItem>,
    idx: usize,
    playlist_item_id: i64,
    track_title: SharedString,
    drag_drop_manager: Entity<DragDropListManager>,
    list_id: SharedString,
}

impl PlaylistTrackItem {
    pub fn new(
        cx: &mut App,
        track_item: Entity<TrackItem>,
        idx: usize,
        playlist_item_id: i64,
        track_title: SharedString,
        drag_drop_manager: Entity<DragDropListManager>,
        list_id: SharedString,
    ) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe(&drag_drop_manager, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                track_item,
                idx,
                playlist_item_id,
                track_title,
                drag_drop_manager,
                list_id,
            }
        })
    }
}

impl Render for PlaylistTrackItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let item_state = DragDropItemState::for_index(&self.drag_drop_manager.read(cx), self.idx);

        let idx = self.idx;
        let list_id = self.list_id.clone();
        let track_title = self.track_title.clone();

        div()
            .id(("playlist-track-item", self.playlist_item_id as u64))
            .w_full()
            .h(px(PLAYLIST_ITEM_HEIGHT))
            .relative()
            .when(item_state.is_being_dragged, |d| d.opacity(0.5))
            .on_drag(DragData::new(idx, list_id), move |_, _, _, cx| {
                DragPreview::new(cx, track_title.clone())
            })
            .drag_over::<DragData>(move |style, _, _, _| style.bg(rgba(0x88888822)))
            .child(self.track_item.clone())
            .child(DropIndicator::with_state(
                item_state.is_drop_target_before,
                item_state.is_drop_target_after,
                theme.button_primary,
            ))
    }
}

pub struct PlaylistView {
    playlist: Arc<Playlist>,
    playlist_track_ids: Arc<Vec<(i64, i64, i64)>>,
    views: Entity<FxHashMap<usize, Entity<PlaylistTrackItem>>>,
    render_counter: Entity<usize>,
    focus_handle: FocusHandle,
    first_render: bool,
    scroll_handle: UniformListScrollHandle,
    drag_drop_manager: Entity<DragDropListManager>,
    list_id: SharedString,
}

impl PlaylistView {
    pub fn new(cx: &mut App, playlist_id: i64) -> Entity<Self> {
        cx.new(|cx| {
            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

            let list_id: SharedString = format!("playlist-{}", playlist_id).into();
            let config = DragDropListConfig::new(list_id.clone(), px(PLAYLIST_ITEM_HEIGHT));
            let drag_drop_manager = DragDropListManager::new(cx, config);

            cx.subscribe(
                &playlist_tracker,
                move |this: &mut Self, _, ev: &PlaylistEvent, cx| {
                    if let PlaylistEvent::PlaylistUpdated(id) = ev
                        && *id == this.playlist.id
                    {
                        this.playlist_track_ids = cx.get_playlist_tracks(this.playlist.id).unwrap();

                        this.views = cx.new(|_| FxHashMap::default());
                        this.render_counter = cx.new(|_| 0);
                    }
                },
            )
            .detach();

            cx.observe(&drag_drop_manager, |_, _, cx| {
                cx.notify();
            })
            .detach();

            let focus_handle = cx.focus_handle();

            cx.register_command(
                ("playlist::export", playlist_id),
                Command::new(
                    Some("Playlist"),
                    "Export Playlist to M3U",
                    Export,
                    Some(focus_handle.clone()),
                ),
            );

            cx.on_release(move |_, cx| {
                cx.unregister_command(("playlist::export", playlist_id));
            })
            .detach();

            Self {
                playlist: cx.get_playlist(playlist_id).unwrap(),
                playlist_track_ids: cx.get_playlist_tracks(playlist_id).unwrap(),
                views: cx.new(|_| FxHashMap::default()),
                render_counter: cx.new(|_| 0),
                focus_handle,
                first_render: true,
                scroll_handle: UniformListScrollHandle::new(),
                drag_drop_manager,
                list_id,
            }
        })
    }

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

impl Render for PlaylistView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        check_drag_cancelled(self.drag_drop_manager.clone(), cx);

        let items_clone = self.playlist_track_ids.clone();
        let views_model = self.views.clone();
        let render_counter = self.render_counter.clone();
        let pl_id = self.playlist.id;
        let playlist_name = self.playlist.name.0.clone();
        let scroll_handle = self.scroll_handle.clone();
        let drag_drop_manager = self.drag_drop_manager.clone();
        let list_id = self.list_id.clone();
        let item_count = items_clone.len();

        if self.first_render {
            self.first_render = false;
            self.focus_handle.focus(window, cx);
        }

        let theme = cx.global::<Theme>();

        div()
            .image_cache(hummingbird_cache(
                ("playlist", self.playlist.id as u64),
                100,
            ))
            .id("playlist-view")
            .track_focus(&self.focus_handle)
            .on_action(move |_: &Export, _, cx| {
                if let Err(err) = export_playlist(cx, pl_id, &playlist_name) {
                    error!("Failed to export playlist: {}", err);
                }
            })
            .pt(px(10.0))
            .flex()
            .flex_col()
            .flex_shrink()
            .overflow_x_hidden()
            .max_w(px(1000.0))
            .h_full()
            .child(
                div()
                    .flex()
                    .overflow_x_hidden()
                    .flex_shrink()
                    .px(px(18.0))
                    .w_full()
                    .child(
                        div()
                            .bg(theme.album_art_background)
                            .shadow_sm()
                            .w(px(160.0))
                            .h(px(160.0))
                            .flex_shrink_0()
                            .rounded(px(4.0))
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                icon(if self.playlist.playlist_type == PlaylistType::System {
                                    STAR
                                } else {
                                    PLAYLIST
                                })
                                .size(px(100.0)),
                            ),
                    )
                    .child(
                        div()
                            .ml(px(18.0))
                            .mt_auto()
                            .flex_shrink()
                            .flex()
                            .flex_col()
                            .w_full()
                            .overflow_x_hidden()
                            .child(
                                div()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_size(rems(2.5))
                                    .line_height(rems(2.75))
                                    .overflow_x_hidden()
                                    .pb(px(10.0))
                                    .w_full()
                                    .text_ellipsis()
                                    .child(self.playlist.name.clone()),
                            )
                            .child(
                                div()
                                    .gap(px(10.0))
                                    .flex()
                                    .child(
                                        button()
                                            .id("playlist-play-button")
                                            .size(ButtonSize::Large)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .intent(ButtonIntent::Primary)
                                            .child(icon(PLAY).size(px(16.0)).my_auto())
                                            .child("Play")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
                                                        )
                                                    })
                                                    .collect();

                                                replace_queue(queue_items, cx);
                                            })),
                                    )
                                    .child(
                                        button()
                                            .id("playlist-add-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .child(icon(CIRCLE_PLUS).size(px(16.0)).my_auto())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
                                                        )
                                                    })
                                                    .collect();

                                                cx.global::<PlaybackInterface>()
                                                    .queue_list(queue_items);
                                            })),
                                    )
                                    .child(
                                        button()
                                            .id("playlist-shuffle-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .child(icon(SHUFFLE).size(px(16.0)).my_auto())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
                                                        )
                                                    })
                                                    .collect();

                                                if !(*cx
                                                    .global::<PlaybackInfo>()
                                                    .shuffling
                                                    .read(cx))
                                                {
                                                    cx.global::<PlaybackInterface>()
                                                        .toggle_shuffle();
                                                }

                                                replace_queue(queue_items, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .id("playlist-list-container")
                    .flex()
                    .w_full()
                    .h_full()
                    .relative()
                    .mt(px(18.0))
                    .on_drag_move::<DragData>(cx.listener(
                        move |this: &mut PlaylistView,
                              event: &DragMoveEvent<DragData>,
                              window,
                              cx| {
                            let scroll_handle: ScrollableHandle = this.scroll_handle.clone().into();

                            let scrolled = handle_drag_move(
                                this.drag_drop_manager.clone(),
                                scroll_handle,
                                event,
                                item_count,
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
                        move |this: &mut PlaylistView, drag_data: &DragData, _, cx| {
                            let playlist_track_ids = this.playlist_track_ids.clone();
                            let playlist_id = this.playlist.id;

                            handle_drop(
                                this.drag_drop_manager.clone(),
                                drag_data,
                                cx,
                                |from_idx, to_idx, cx| {
                                    let item_id = playlist_track_ids[from_idx].0;

                                    let new_position = if to_idx < playlist_track_ids.len() {
                                        let target_item_id = playlist_track_ids[to_idx].0;
                                        let target_item =
                                            cx.get_playlist_item(target_item_id).unwrap();
                                        target_item.position
                                    } else {
                                        let last_item_id =
                                            playlist_track_ids[playlist_track_ids.len() - 1].0;
                                        let last_item = cx.get_playlist_item(last_item_id).unwrap();
                                        last_item.position + 1
                                    };

                                    if let Err(e) = cx.move_playlist_item(item_id, new_position) {
                                        error!("Failed to move playlist item: {}", e);
                                        return;
                                    }

                                    let tracker = cx.global::<Models>().playlist_tracker.clone();
                                    tracker.update(cx, |_, cx| {
                                        cx.emit(PlaylistEvent::PlaylistUpdated(playlist_id));
                                    });
                                },
                            );
                            cx.notify();
                        },
                    ))
                    .child(
                        uniform_list("playlist-list", items_clone.len(), move |range, _, cx| {
                            let start = range.start;
                            let is_templ_render = range.start == 0 && range.end == 1;

                            let items = &items_clone[range];

                            items
                                .iter()
                                .enumerate()
                                .map(|(idx, item)| {
                                    let idx = idx + start;

                                    if !is_templ_render {
                                        prune_views(&views_model, &render_counter, idx, cx);
                                    }

                                    let drag_drop_manager = drag_drop_manager.clone();
                                    let list_id = list_id.clone();
                                    let playlist_item_id = item.0;
                                    let track_id = item.1;

                                    div().h(px(PLAYLIST_ITEM_HEIGHT)).child(
                                        create_or_retrieve_view(
                                            &views_model,
                                            idx,
                                            move |cx| {
                                                let track = cx.get_track_by_id(track_id).unwrap();
                                                let track_title: SharedString =
                                                    track.title.clone().into();

                                                let track_item = TrackItem::new(
                                                    cx,
                                                    Arc::try_unwrap(track).unwrap(),
                                                    false,
                                                    ArtistNameVisibility::Always,
                                                    TrackItemLeftField::Art,
                                                    Some(TrackPlaylistInfo {
                                                        id: pl_id,
                                                        item_id: playlist_item_id,
                                                    }),
                                                );

                                                PlaylistTrackItem::new(
                                                    cx,
                                                    track_item,
                                                    idx,
                                                    playlist_item_id,
                                                    track_title,
                                                    drag_drop_manager,
                                                    list_id,
                                                )
                                            },
                                            cx,
                                        ),
                                    )
                                })
                                .collect()
                        })
                        .w_full()
                        .h_full()
                        .flex()
                        .flex_col()
                        .border_color(theme.border_color)
                        .border_t_1()
                        .track_scroll(&scroll_handle),
                    )
                    .child(floating_scrollbar("playlist", scroll_handle, RightPad::Pad)),
            )
    }
}
