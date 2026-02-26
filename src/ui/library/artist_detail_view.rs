use std::{rc::Rc, sync::Arc};

use cntp_i18n::tr;
use gpui::*;
use prelude::FluentBuilder;
use rustc_hash::FxHashMap;

use crate::{
    library::{
        db::{LibraryAccess, LikedTrackSortMethod},
        types::{Album, DBString, Track, table::AlbumColumn},
    },
    playback::{
        interface::{PlaybackInterface, replace_queue},
        queue::QueueItemData,
        thread::PlaybackState,
    },
    ui::{
        caching::hummingbird_cache,
        components::{
            button::{ButtonIntent, ButtonSize, button},
            dropdown::{DropdownOption, DropdownState, dropdown},
            icons::{CIRCLE_PLUS, PAUSE, PLAY, SHUFFLE, SORT, icon},
            scrollbar::{RightPad, floating_scrollbar},
            table::{grid_item::GridItem, table_data::TABLE_MAX_WIDTH},
            uniform_grid::uniform_grid,
        },
        global_actions::PlayPause,
        library::track_listing::{
            ArtistNameVisibility,
            track_item::{TrackItem, TrackItemLeftField},
        },
        models::{Models, PlaybackInfo, PlaylistEvent},
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

use super::ViewSwitchMessage;

type GridHandler = dyn Fn(&mut App, &(u32, String)) + 'static;

pub struct ArtistDetailView {
    artist_name: Option<DBString>,
    album_ids: Vec<(u32, String)>,
    liked_track_items: Vec<Entity<TrackItem>>,
    all_tracks: Arc<Vec<Track>>,
    liked_tracks: Arc<Vec<Track>>,
    scroll_handle: ScrollHandle,
    grid_views: Entity<FxHashMap<usize, Entity<GridItem<Album, AlbumColumn>>>>,
    grid_render_counter: Entity<usize>,
    nav_model: Entity<super::NavigationHistory>,
    liked_sort: LikedTrackSortMethod,
    liked_sort_dropdown: Entity<DropdownState>,
}

impl ArtistDetailView {
    pub(super) fn new(
        cx: &mut App,
        artist_id: i64,
        nav_model: Entity<super::NavigationHistory>,
    ) -> Entity<Self> {
        let view: Entity<Self> = cx.new(|cx| {
            let artist = cx.get_artist_by_id(artist_id).ok();
            let artist_name = artist.as_ref().and_then(|a| a.name.clone());

            let album_ids = cx.list_albums_by_artist(artist_id).unwrap_or_default();

            let all_tracks = cx
                .get_all_tracks_by_artist(artist_id)
                .unwrap_or_else(|_| Arc::new(Vec::new()));

            let liked_tracks = cx
                .get_liked_tracks_by_artist(artist_id, LikedTrackSortMethod::RecentlyAdded)
                .unwrap_or_else(|_| Arc::new(Vec::new()));

            let liked_track_items: Vec<Entity<TrackItem>> = liked_tracks
                .iter()
                .map(|track| {
                    TrackItem::new(
                        cx,
                        track.clone(),
                        false,
                        ArtistNameVisibility::OnlyIfDifferent(artist_name.clone()),
                        TrackItemLeftField::Art,
                        None,
                        false,
                        None,
                    )
                })
                .collect();

            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

            cx.subscribe(&playlist_tracker, move |this: &mut Self, _, ev, cx| {
                if let PlaylistEvent::PlaylistUpdated(1) = ev {
                    let liked_tracks = cx
                        .get_liked_tracks_by_artist(artist_id, this.liked_sort)
                        .unwrap_or_else(|_| Arc::new(Vec::new()));

                    this.liked_tracks = liked_tracks.clone();
                    this.liked_track_items = liked_tracks
                        .iter()
                        .map(|track| {
                            TrackItem::new(
                                cx,
                                track.clone(),
                                false,
                                ArtistNameVisibility::OnlyIfDifferent(this.artist_name.clone()),
                                TrackItemLeftField::Art,
                                None,
                                false,
                                None,
                            )
                        })
                        .collect();

                    cx.notify();
                }
            })
            .detach();

            let grid_views = cx.new(|_| FxHashMap::default());
            let grid_render_counter = cx.new(|_| 0usize);

            let dropdown_options = vec![
                DropdownOption::new(
                    "recently_added",
                    tr!("SORT_RECENTLY_ADDED", "Recently Added"),
                ),
                DropdownOption::new("title", tr!("SORT_TITLE", "Title")),
                DropdownOption::new("release_order", tr!("SORT_RELEASE_ORDER", "Release Order")),
            ];

            let focus_handle = cx.focus_handle();
            let liked_sort_dropdown = dropdown(cx, dropdown_options, 0, focus_handle);

            ArtistDetailView {
                artist_name,
                album_ids,
                liked_track_items,
                all_tracks,
                liked_tracks: liked_tracks.clone(),
                scroll_handle: ScrollHandle::new(),
                grid_views,
                grid_render_counter,
                nav_model,
                liked_sort: LikedTrackSortMethod::RecentlyAdded,
                liked_sort_dropdown,
            }
        });

        let weak_view = view.downgrade();
        view.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
            this.liked_sort_dropdown.update(cx, |dropdown: &mut DropdownState, _| {
                dropdown.set_width(px(200.0));
                dropdown.set_on_change(move |idx, _, _, cx| {
                    let sort_method = match idx {
                        0 => LikedTrackSortMethod::RecentlyAdded,
                        1 => LikedTrackSortMethod::TitleAsc,
                        2 => LikedTrackSortMethod::ReleaseOrder,
                        _ => unreachable!(),
                    };

                    if let Some(view) = weak_view.upgrade() {
                        view.update(cx, |this: &mut ArtistDetailView, cx: &mut Context<ArtistDetailView>| {
                            this.update_liked_sort(sort_method, artist_id, cx);
                        });
                    }
                });
            });
        });

        view
    }

    pub fn update_liked_sort(
        &mut self,
        sort_method: LikedTrackSortMethod,
        artist_id: i64,
        cx: &mut Context<Self>,
    ) {
        if self.liked_sort == sort_method {
            return;
        }

        self.liked_sort = sort_method;

        let liked_tracks = cx
            .get_liked_tracks_by_artist(artist_id, self.liked_sort)
            .unwrap_or_else(|_| Arc::new(Vec::new()));

        self.liked_tracks = liked_tracks.clone();
        self.liked_track_items = liked_tracks
            .iter()
            .map(|track: &Track| {
                TrackItem::new(
                    cx,
                    track.clone(),
                    false,
                    ArtistNameVisibility::OnlyIfDifferent(self.artist_name.clone()),
                    TrackItemLeftField::Art,
                    None,
                    false,
                    None,
                )
            })
            .collect();

        cx.notify();
    }
}

impl Render for ArtistDetailView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let scroll_handle = self.scroll_handle.clone();
        let settings = cx
            .global::<crate::settings::SettingsGlobal>()
            .model
            .read(cx);
        let full_width = settings.interface.full_width_library;
        let grid_min_item_width = crate::settings::interface::clamp_grid_min_item_width(
            settings.interface.grid_min_item_width,
        );

        let album_count = self.album_ids.len();
        let album_ids = self.album_ids.clone();
        let grid_views_model = self.grid_views.clone();
        let grid_render_counter = self.grid_render_counter.clone();
        let nav_model = self.nav_model.clone();

        let is_playing =
            cx.global::<PlaybackInfo>().playback_state.read(cx) == &PlaybackState::Playing;

        let current_track_in_artist = cx
            .global::<PlaybackInfo>()
            .current_track
            .read(cx)
            .clone()
            .is_some_and(|current_track| {
                self.all_tracks
                    .iter()
                    .any(|track| current_track == track.location)
            });

        let current_track_in_liked = cx
            .global::<PlaybackInfo>()
            .current_track
            .read(cx)
            .clone()
            .is_some_and(|current_track| {
                self.liked_tracks
                    .iter()
                    .any(|track| current_track == track.location)
            });

        let liked_track_header = if !self.liked_track_items.is_empty() {
            Some(
                div()
                    .px(px(18.0))
                    .pt(px(16.0))
                    .pb(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_size(px(18.0))
                            .my_auto()
                            .child(tr!("ARTIST_LIKED_TRACKS", "Liked Tracks")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_between()
                            .pb(px(13.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(10.0))
                                    .child(
                                        button()
                                            .id("artist-liked-play-button")
                                            .size(ButtonSize::Large)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .intent(ButtonIntent::Primary)
                                            .when(!current_track_in_liked, |this| {
                                                this.on_click(cx.listener(
                                                    |this: &mut ArtistDetailView, _, _, cx| {
                                                        let queue_items = this
                                                            .liked_tracks
                                                            .iter()
                                                            .map(|track| {
                                                                QueueItemData::new(
                                                                    cx,
                                                                    track.location.clone(),
                                                                    Some(track.id),
                                                                    track.album_id,
                                                                )
                                                            })
                                                            .collect();

                                                        replace_queue(queue_items, cx)
                                                    },
                                                ))
                                            })
                                            .when(current_track_in_liked, |button| {
                                                button.on_click(|_, window, cx| {
                                                    window.dispatch_action(Box::new(PlayPause), cx);
                                                })
                                            })
                                            .child(
                                                icon(if current_track_in_liked && is_playing {
                                                    PAUSE
                                                } else {
                                                    PLAY
                                                })
                                                .size(px(16.0))
                                                .my_auto(),
                                            )
                                            .child(div().child(
                                                if current_track_in_liked && is_playing {
                                                    tr!("PAUSE")
                                                } else {
                                                    tr!("PLAY")
                                                },
                                            )),
                                    )
                                    .child(
                                        button()
                                            .id("artist-liked-add-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .on_click(cx.listener(
                                                |this: &mut ArtistDetailView, _, _, cx| {
                                                    let queue_items = this
                                                        .liked_tracks
                                                        .iter()
                                                        .map(|track| {
                                                            QueueItemData::new(
                                                                cx,
                                                                track.location.clone(),
                                                                Some(track.id),
                                                                track.album_id,
                                                            )
                                                        })
                                                        .collect();

                                                    cx.global::<PlaybackInterface>()
                                                        .queue_list(queue_items);
                                                },
                                            ))
                                            .child(icon(CIRCLE_PLUS).size(px(16.0)).my_auto()),
                                    )
                                    .child(
                                        button()
                                            .id("artist-liked-shuffle-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .on_click(cx.listener(
                                                |this: &mut ArtistDetailView, _, _, cx| {
                                                    let queue_items = this
                                                        .liked_tracks
                                                        .iter()
                                                        .map(|track| {
                                                            QueueItemData::new(
                                                                cx,
                                                                track.location.clone(),
                                                                Some(track.id),
                                                                track.album_id,
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

                                                    replace_queue(queue_items, cx)
                                                },
                                            ))
                                            .child(icon(SHUFFLE).size(px(16.0)).my_auto()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .items_center()
                                    .child(
                                        icon(SORT).text_color(theme.text_secondary).size(px(20.0)),
                                    )
                                    .child(self.liked_sort_dropdown.clone()),
                            ),
                    ),
            )
        } else {
            None
        };

        div()
            .flex()
            .w_full()
            .max_h_full()
            .relative()
            .overflow_hidden()
            .mt(px(10.0))
            .border_t_1()
            .border_color(theme.border_color)
            .when(!full_width, |this| this.max_w(px(TABLE_MAX_WIDTH)))
            .child(
                div()
                    .id("artist-detail-view")
                    .overflow_y_scroll()
                    .track_scroll(&scroll_handle)
                    .w_full()
                    .flex_shrink()
                    .overflow_x_hidden()
                    // Artist name header
                    .child(
                        div()
                            .pt(px(18.0))
                            .px(px(18.0))
                            .w_full()
                            .child(
                                div()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_size(rems(2.5))
                                    .line_height(rems(2.75))
                                    .overflow_x_hidden()
                                    .pb(px(10.0))
                                    .w_full()
                                    .text_ellipsis()
                                    .when_some(self.artist_name.clone(), |this, name| {
                                        this.child(name)
                                    }),
                            )
                            .when(!self.all_tracks.is_empty(), |this| {
                                this.child(
                                    div()
                                        .gap(px(10.0))
                                        .flex()
                                        .flex_row()
                                        .pb(px(10.0))
                                        .child(
                                            button()
                                                .id("artist-play-button")
                                                .size(ButtonSize::Large)
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .intent(ButtonIntent::Primary)
                                                .when(!current_track_in_artist, |this| {
                                                    this.on_click(cx.listener(
                                                        |this: &mut ArtistDetailView, _, _, cx| {
                                                            let queue_items = this
                                                                .all_tracks
                                                                .iter()
                                                                .map(|track| {
                                                                    QueueItemData::new(
                                                                        cx,
                                                                        track.location.clone(),
                                                                        Some(track.id),
                                                                        track.album_id,
                                                                    )
                                                                })
                                                                .collect();

                                                            replace_queue(queue_items, cx)
                                                        },
                                                    ))
                                                })
                                                .when(current_track_in_artist, |button| {
                                                    button.on_click(|_, window, cx| {
                                                        window.dispatch_action(
                                                            Box::new(PlayPause),
                                                            cx,
                                                        );
                                                    })
                                                })
                                                .child(
                                                    icon(
                                                        if current_track_in_artist && is_playing {
                                                            PAUSE
                                                        } else {
                                                            PLAY
                                                        },
                                                    )
                                                    .size(px(16.0))
                                                    .my_auto(),
                                                )
                                                .child(div().child(
                                                    if current_track_in_artist && is_playing {
                                                        tr!("PAUSE")
                                                    } else {
                                                        tr!("PLAY")
                                                    },
                                                )),
                                        )
                                        .child(
                                            button()
                                                .id("artist-add-button")
                                                .size(ButtonSize::Large)
                                                .flex_none()
                                                .on_click(cx.listener(
                                                    |this: &mut ArtistDetailView, _, _, cx| {
                                                        let queue_items = this
                                                            .all_tracks
                                                            .iter()
                                                            .map(|track| {
                                                                QueueItemData::new(
                                                                    cx,
                                                                    track.location.clone(),
                                                                    Some(track.id),
                                                                    track.album_id,
                                                                )
                                                            })
                                                            .collect();

                                                        cx.global::<PlaybackInterface>()
                                                            .queue_list(queue_items);
                                                    },
                                                ))
                                                .child(icon(CIRCLE_PLUS).size(px(16.0)).my_auto()),
                                        )
                                        .child(
                                            button()
                                                .id("artist-shuffle-button")
                                                .size(ButtonSize::Large)
                                                .flex_none()
                                                .on_click(cx.listener(
                                                    |this: &mut ArtistDetailView, _, _, cx| {
                                                        let queue_items = this
                                                            .all_tracks
                                                            .iter()
                                                            .map(|track| {
                                                                QueueItemData::new(
                                                                    cx,
                                                                    track.location.clone(),
                                                                    Some(track.id),
                                                                    track.album_id,
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

                                                        replace_queue(queue_items, cx)
                                                    },
                                                ))
                                                .child(icon(SHUFFLE).size(px(16.0)).my_auto()),
                                        ),
                                )
                            }),
                    )
                    .when(album_count > 0, |this| {
                        let handler: Option<Rc<GridHandler>> = Some(Rc::new(move |cx, id| {
                            nav_model.update(cx, |_, cx| {
                                cx.emit(ViewSwitchMessage::Release(id.0 as i64));
                            });
                        }));
                        let grid_padding = 8.0_f32;

                        this.child(
                            div()
                                .px(px(18.0))
                                .pt(px(8.0))
                                .mb(px(-6.0))
                                .font_weight(FontWeight::BOLD)
                                .text_size(px(18.0))
                                .child(tr!("ARTIST_ALBUMS", "Albums")),
                        )
                        .child(
                            div().px(px(grid_padding)).w_full().child(
                                uniform_grid(
                                    "artist-albums-grid",
                                    album_count,
                                    None,
                                    move |idx, _, cx| {
                                        prune_views(
                                            &grid_views_model,
                                            &grid_render_counter,
                                            idx,
                                            cx,
                                        );

                                        let item_id = album_ids[idx].clone();

                                        let view = create_or_retrieve_view(
                                            &grid_views_model,
                                            idx,
                                            |cx| {
                                                GridItem::<Album, AlbumColumn>::new(
                                                    cx,
                                                    item_id,
                                                    handler.clone(),
                                                )
                                                .unwrap()
                                            },
                                            cx,
                                        );

                                        div()
                                            .image_cache(hummingbird_cache(
                                                ("artist-album-grid", idx + 1),
                                                1,
                                            ))
                                            .size_full()
                                            .child(view)
                                            .into_any_element()
                                    },
                                )
                                .min_item_width(px(grid_min_item_width))
                                .gap(px(0.0))
                                .py(px(grid_padding))
                                .auto_height(),
                            ),
                        )
                    })
                    .when_some(liked_track_header, |this, header| {
                        this.child(header).child(
                            div()
                                .w_full()
                                .border_t_1()
                                .border_color(theme.border_color)
                                .image_cache(retain_all("artist_liked_tracks_cache"))
                                .children(
                                    self.liked_track_items
                                        .iter()
                                        .map(|item| div().h(px(40.0)).child(item.clone())),
                                ),
                        )
                    }),
            )
            .child(floating_scrollbar(
                "artist_detail_scrollbar",
                scroll_handle,
                RightPad::Pad,
            ))
    }
}
