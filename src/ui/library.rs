use album_view::AlbumView;
use gpui::*;

pub mod album_view;

pub struct Library {
    pub album_view: View<AlbumView>,
}

impl Library {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let album_view = AlbumView::new(cx);
            Library { album_view }
        })
    }
}

impl Render for Library {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().child(self.album_view.clone())
    }
}
