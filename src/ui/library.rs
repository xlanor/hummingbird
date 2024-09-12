use album_view::AlbumView;
use gpui::*;
use release_view::ReleaseView;

pub mod album_view;
pub mod release_view;

enum LibraryView {
    Album(View<AlbumView>),
    Release(View<ReleaseView>),
}

pub struct Library {
    view: LibraryView,
    switcher_model: Model<ViewSwitchDummy>,
}

enum ViewSwitchMessage {
    Albums,
    Release(i64),
}

pub struct ViewSwitchDummy;

impl EventEmitter<ViewSwitchMessage> for ViewSwitchDummy {}

impl Library {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let switcher_model = cx.new_model(|_| ViewSwitchDummy);
            let view = LibraryView::Album(AlbumView::new(cx, switcher_model.clone()));

            let switcher_model_copy = switcher_model.clone();

            cx.subscribe(&switcher_model, move |this: &mut Library, _, e, cx| {
                this.view = match e {
                    ViewSwitchMessage::Albums => {
                        LibraryView::Album(AlbumView::new(cx, switcher_model_copy.clone()))
                    }
                    ViewSwitchMessage::Release(id) => {
                        LibraryView::Release(ReleaseView::new(cx, *id, switcher_model_copy.clone()))
                    }
                };

                cx.notify();
            })
            .detach();

            Library {
                view,
                switcher_model,
            }
        })
    }
}

impl Render for Library {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().w_full().h_full().flex().child(match &self.view {
            LibraryView::Album(album_view) => album_view.clone().into_any_element(),
            LibraryView::Release(release_view) => release_view.clone().into_any_element(),
        })
    }
}
