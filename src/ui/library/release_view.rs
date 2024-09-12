use std::sync::Arc;

use gpui::*;

use crate::{
    library::{
        db::LibraryAccess,
        types::{Album, Artist, Track},
    },
    ui::models::TransferDummy,
};

use super::ViewSwitchDummy;

pub struct ReleaseView {
    album: Arc<Album>,
    image_transfer_model: Model<TransferDummy>,
    image: Option<Arc<RenderImage>>,
    artist: Option<Arc<Artist>>,
    tracks: Vec<Arc<Track>>,
    view_switcher_model: Model<ViewSwitchDummy>,
}

impl ReleaseView {
    pub fn new<V: 'static>(
        cx: &mut ViewContext<V>,
        album_id: i64,
        view_switcher_model: Model<ViewSwitchDummy>,
    ) -> View<Self> {
        cx.new_view(|cx| {
            let image_transfer_model = cx.new_model(|_| TransferDummy);
            let image = None;
            let artist = None;
            // TODO: error handling
            let album = cx
                .get_album_by_id(album_id)
                .expect("Failed to retrieve album");
            let tracks = Vec::new();

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
        div().child("release view")
    }
}
