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
        div().child(
            div()
                .flex()
                .child(
                    div()
                        .rounded(px(8.0))
                        .bg(rgb(0x4b5563))
                        .shadow_sm()
                        .w(px(120.0))
                        .h(px(120.0))
                        .ml(px(12.0))
                        .my(px(8.0))
                        .flex_shrink_0()
                        .when(self.image.is_some(), |div| {
                            div.child(
                                img(self.image.clone().unwrap())
                                    .w(px(120.0))
                                    .h(px(120.0))
                                    .rounded(px(8.0)),
                            )
                        }),
                )
                .child(
                    div()
                        .ml(px(12.0))
                        .my_auto()
                        .child(div().font_weight(FontWeight::SEMIBOLD).when_some(
                            self.artist.as_ref().map(|v| v.name.clone()),
                            |this, artist| this.child(artist.unwrap()),
                        ))
                        .child(
                            div()
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .text_size(rems(2.5))
                                .line_height(rems(2.5))
                                .child(self.album.title.clone()),
                        ),
                ),
        )
    }
}
