use std::{rc::Rc, sync::Arc};

use cntp_i18n::tr;
use gpui::*;
use prelude::FluentBuilder;
use rustc_hash::FxHashMap;

use crate::{
    library::{
        db::LibraryAccess,
        types::{Album, DBString, table::AlbumColumn},
    },
    ui::{
        caching::hummingbird_cache,
        components::{
            scrollbar::{RightPad, floating_scrollbar},
            table::{grid_item::GridItem, table_data::TABLE_MAX_WIDTH},
            uniform_grid::uniform_grid,
        },
        library::track_listing::{
            ArtistNameVisibility,
            track_item::{TrackItem, TrackItemLeftField},
        },
        models::Models,
        models::PlaylistEvent,
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
    scroll_handle: ScrollHandle,
    grid_views: Entity<FxHashMap<usize, Entity<GridItem<Album, AlbumColumn>>>>,
    grid_render_counter: Entity<usize>,
    nav_model: Entity<super::NavigationHistory>,
}

impl ArtistDetailView {
    pub(super) fn new(
        cx: &mut App,
        artist_id: i64,
        nav_model: Entity<super::NavigationHistory>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let artist = cx.get_artist_by_id(artist_id).ok();
            let artist_name = artist.as_ref().and_then(|a| a.name.clone());

            let album_ids = cx.list_albums_by_artist(artist_id).unwrap_or_default();

            let liked_tracks = cx
                .get_liked_tracks_by_artist(artist_id)
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
                        .get_liked_tracks_by_artist(artist_id)
                        .unwrap_or_else(|_| Arc::new(Vec::new()));

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

            ArtistDetailView {
                artist_name,
                album_ids,
                liked_track_items,
                scroll_handle: ScrollHandle::new(),
                grid_views,
                grid_render_counter,
                nav_model,
            }
        })
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
                        div().pt(px(18.0)).px(px(18.0)).w_full().child(
                            div()
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .text_size(rems(2.5))
                                .line_height(rems(2.75))
                                .overflow_x_hidden()
                                .pb(px(10.0))
                                .w_full()
                                .text_ellipsis()
                                .when_some(self.artist_name.clone(), |this, name| this.child(name)),
                        ),
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
                                .pb(px(4.0))
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
                    .when(!self.liked_track_items.is_empty(), |this| {
                        this.child(
                            div()
                                .px(px(18.0))
                                .pt(px(16.0))
                                .pb(px(4.0))
                                .font_weight(FontWeight::BOLD)
                                .text_size(px(18.0))
                                .child(tr!("ARTIST_LIKED_TRACKS", "Liked Tracks")),
                        )
                        .child(
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
