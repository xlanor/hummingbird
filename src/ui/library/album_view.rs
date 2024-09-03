use gpui::*;
use tracing::error;

use crate::library::db::{AlbumSortMethod, LibraryAccess};

#[derive(Clone)]
pub struct AlbumView {
    album_ids: Model<Vec<(u32, String)>>,
}

impl AlbumView {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let album_ids = cx.list_albums(AlbumSortMethod::TitleAsc);
            match album_ids {
                Ok(album_ids) => AlbumView {
                    album_ids: cx.new_model(|_| album_ids),
                },
                Err(e) => {
                    error!("Failed to retrieve album IDs from SQLite: {:?}", e);
                    error!("Returning empty album view");
                    AlbumView {
                        album_ids: cx.new_model(|_| Vec::new()),
                    }
                }
            }
        })
    }
}

impl Render for AlbumView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().child("albumview")
    }
}
