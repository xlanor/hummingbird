use std::{collections::VecDeque, sync::Arc};

use gpui::*;
use prelude::FluentBuilder;
use tracing::debug;

use crate::{
    data::{
        events::{ImageLayout, ImageType},
        interface::GPUIDataInterface,
    },
    library::{
        db::{AlbumMethod, LibraryAccess},
        types::{Album, Artist, Track},
    },
    playback::interface::{replace_queue, GPUIPlaybackInterface},
    ui::models::{Models, TransferDummy},
};

use super::ViewSwitchMessage;

pub struct ReleaseView {
    album: Arc<Album>,
    image_transfer_model: Model<TransferDummy>,
    image: Option<Arc<RenderImage>>,
    artist: Option<Arc<Artist>>,
    tracks: Arc<Vec<Track>>,
    view_switcher_model: Model<VecDeque<ViewSwitchMessage>>,
    track_list_state: ListState,
}

impl ReleaseView {
    pub(super) fn new<V: 'static>(
        cx: &mut ViewContext<V>,
        album_id: i64,
        view_switcher_model: Model<VecDeque<ViewSwitchMessage>>,
    ) -> View<Self> {
        cx.new_view(|cx| {
            let image = None;
            // TODO: error handling
            let album = cx
                .get_album_by_id(album_id, AlbumMethod::Cached)
                .expect("Failed to retrieve album");
            let tracks = cx
                .list_tracks_in_album(album_id)
                .expect("Failed to retrieve tracks");
            let artist = cx.get_artist_by_id(album.artist_id).ok();

            let image_transfer_model = cx.global::<Models>().image_transfer_model.clone();

            cx.subscribe(
                &image_transfer_model,
                move |this: &mut ReleaseView, _, image, cx| {
                    if image.0 == ImageType::AlbumArt(album_id) {
                        debug!("captured decoded image for album ID: {}", album_id);
                        this.image = Some(image.1.clone());
                        cx.notify();
                    }
                },
            )
            .detach();

            if let Some(image) = album.image.clone() {
                cx.global::<GPUIDataInterface>().decode_image(
                    image,
                    ImageType::AlbumArt(album_id),
                    ImageLayout::BGR,
                    false,
                );
            }

            let tracks_clone = tracks.clone();

            let state =
                ListState::new(tracks.len(), ListAlignment::Top, px(25.0), move |idx, _| {
                    TrackItem {
                        track: tracks_clone[idx].clone(),
                        is_start: if idx > 0 {
                            if let Some(track) = tracks_clone.get(idx - 1) {
                                track.disc_number != tracks_clone[idx].disc_number
                            } else {
                                true
                            }
                        } else {
                            true
                        },
                        tracks: tracks_clone.clone(),
                    }
                    .into_any_element()
                });

            ReleaseView {
                album,
                image_transfer_model,
                image,
                artist,
                tracks,
                view_switcher_model,
                track_list_state: state,
            }
        })
    }
}

impl Render for ReleaseView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let release_info = {
            let mut info = String::new();

            if let Some(label) = &self.album.label {
                info += label;
            }

            if self.album.label.is_some() && self.album.catalog_number.is_some() {
                info += " • ";
            }

            if let Some(catalog_number) = &self.album.catalog_number {
                info += catalog_number;
            }

            info
        };

        div()
            .mt(px(24.0))
            .w_full()
            .flex_shrink()
            .overflow_x_hidden()
            .h_full()
            .max_w(px(1000.0))
            .mx_auto()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_shrink()
                    .flex()
                    .overflow_x_hidden()
                    .px(px(24.0))
                    .w_full()
                    .child(
                        div()
                            .rounded(px(4.0))
                            .bg(rgb(0x4b5563))
                            .shadow_sm()
                            .w(px(160.0))
                            .h(px(160.0))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .when(self.image.is_some(), |div| {
                                div.child(
                                    img(self.image.clone().unwrap())
                                        .min_w(px(160.0))
                                        .min_h(px(160.0))
                                        .max_w(px(160.0))
                                        .max_h(px(160.0))
                                        .overflow_hidden()
                                        .flex()
                                        // TODO: Ideally this should be ObjectFit::Cover, but for
                                        // some reason that makes the element bigger
                                        // FIXME: Is this a GPUI bug?
                                        .object_fit(ObjectFit::Fill)
                                        .rounded(px(4.0)),
                                )
                            }),
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
                            .child(div().font_weight(FontWeight::SEMIBOLD).when_some(
                                self.artist.as_ref().map(|v| v.name.clone()),
                                |this, artist| this.child(artist.unwrap()),
                            ))
                            .child(
                                div()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_size(rems(2.5))
                                    .line_height(rems(2.75))
                                    .overflow_x_hidden()
                                    .min_w_0()
                                    .text_ellipsis()
                                    .child(self.album.title.clone()),
                            )
                            .child(
                                // TODO: add shuffle, add to queue buttons
                                div().flex().flex_row().child(
                                    div()
                                        .mt(px(10.0))
                                        .id("play-button")
                                        .bg(rgb(0x1f2937))
                                        .border_1()
                                        .border_color(rgb(0x374151))
                                        .rounded(px(4.0))
                                        .pl(px(11.0))
                                        .pr(px(10.0))
                                        .py(px(3.0))
                                        .shadow_sm()
                                        .text_sm()
                                        .flex()
                                        .cursor_pointer()
                                        .hover(|this| this.bg(rgb(0x374151)))
                                        .font_weight(FontWeight::BOLD)
                                        .active(|style| style.bg(rgb(0x111827)))
                                        .on_click(cx.listener(|this: &mut ReleaseView, _, cx| {
                                            let paths = this
                                                .tracks
                                                .iter()
                                                .map(|track| track.location.clone())
                                                .collect();

                                            replace_queue(paths, cx)
                                        }))
                                        .gap(px(6.0))
                                        .child(div().font_family("Font Awesome 6 Free").child(""))
                                        .child(div().child("Play")),
                                ),
                            ),
                    ),
            )
            .child(
                list(self.track_list_state.clone())
                    .w_full()
                    .flex()
                    .h_full()
                    .flex_col()
                    .mx_auto(),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .text_sm()
                    .ml(px(24.0))
                    .mt(px(24.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(0xd1d5db))
                    .when(!release_info.is_empty(), |this| {
                        this.child(div().child(release_info))
                    })
                    .when_some(self.album.release_date, |this, date| {
                        this.child(div().child(format!("Released {}", date.format("%B %e, %Y"))))
                    })
                    .when_some(self.album.isrc.as_ref(), |this, isrc| {
                        this.child(div().child(format!("{}", isrc)))
                    }),
            )
    }
}

#[derive(IntoElement)]
struct TrackItem {
    pub track: Track,
    pub is_start: bool,
    pub tracks: Arc<Vec<Track>>,
}

impl RenderOnce for TrackItem {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let tracks = self.tracks.clone();
        let track_id = self.track.id;
        div()
            .flex()
            .flex_col()
            .w_full()
            .id(self.track.id as usize)
            .on_click(move |_, cx| {
                let paths = tracks.iter().map(|track| track.location.clone()).collect();

                replace_queue(paths, cx);

                let playback_interface = cx.global::<GPUIPlaybackInterface>();
                playback_interface.jump(tracks.iter().position(|t| t.id == track_id).unwrap())
            })
            .when(self.is_start, |this| {
                this.child(
                    div()
                        .text_color(rgb(0xd1d5db))
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .px(px(24.0))
                        .border_b_1()
                        .w_full()
                        .border_color(rgb(0x1e293b))
                        .mt(px(24.0))
                        .pb(px(6.0))
                        .child(format!(
                            "DISC {}",
                            self.track.disc_number.unwrap_or_default()
                        )),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .w_full()
                    .border_color(rgb(0x1e293b))
                    .cursor_pointer()
                    .px(px(24.0))
                    .py(px(6.0))
                    .hover(|this| this.bg(rgb(0x1e293b)))
                    .max_w_full()
                    .child(
                        div()
                            .w(px(62.0))
                            .child(format!("{}", self.track.track_number.unwrap_or_default())),
                    )
                    .child(div().font_weight(FontWeight::BOLD).child(self.track.title))
                    .child(div().ml_auto().child(format!(
                        "{}:{:02}",
                        self.track.duration / 60,
                        self.track.duration % 60
                    ))),
            )
    }
}
