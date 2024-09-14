use std::sync::Arc;

use gpui::*;
use prelude::FluentBuilder;
use tracing::debug;

use crate::{
    data::{
        events::{ImageLayout, ImageType},
        interface::GPUIDataInterface,
    },
    library::{
        db::LibraryAccess,
        types::{Album, Artist, Track},
    },
    playback::interface::{replace_queue, GPUIPlaybackInterface},
    ui::models::{Models, TransferDummy},
};

use super::ViewSwitchDummy;

pub struct ReleaseView {
    album: Arc<Album>,
    image_transfer_model: Model<TransferDummy>,
    image: Option<Arc<RenderImage>>,
    artist: Option<Arc<Artist>>,
    tracks: Arc<Vec<Track>>,
    view_switcher_model: Model<ViewSwitchDummy>,
}

impl ReleaseView {
    pub fn new<V: 'static>(
        cx: &mut ViewContext<V>,
        album_id: i64,
        view_switcher_model: Model<ViewSwitchDummy>,
    ) -> View<Self> {
        cx.new_view(|cx| {
            let image = None;
            // TODO: error handling
            let album = cx
                .get_album_by_id(album_id)
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

            ReleaseView {
                album,
                image_transfer_model,
                image,
                artist,
                tracks,
                view_switcher_model,
            }
        })
    }
}

impl Render for ReleaseView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .mt(px(24.0))
            .w_full()
            .flex_shrink()
            .overflow_x_hidden()
            .w_full()
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
                                        .id("play-button-awefg")
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
    }
}
