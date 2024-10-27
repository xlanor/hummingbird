use std::collections::VecDeque;

use album_view::AlbumView;
use gpui::*;
use navigation::NavigationView;
use release_view::ReleaseView;
use tracing::debug;

mod album_view;
mod navigation;
mod release_view;

#[derive(Clone)]
enum LibraryView {
    Album(View<AlbumView>),
    Release(View<ReleaseView>),
}

pub struct Library {
    view: LibraryView,
    navigation_view: View<NavigationView>,
}

#[derive(Clone, Copy, Debug)]
enum ViewSwitchMessage {
    Albums,
    Release(i64),
    Back,
}

impl EventEmitter<ViewSwitchMessage> for VecDeque<ViewSwitchMessage> {}

fn make_view(
    message: &ViewSwitchMessage,
    cx: &mut ViewContext<'_, Library>,
    model: Model<VecDeque<ViewSwitchMessage>>,
) -> LibraryView {
    match message {
        ViewSwitchMessage::Albums => LibraryView::Album(AlbumView::new(cx, model.clone())),
        ViewSwitchMessage::Release(id) => LibraryView::Release(ReleaseView::new(cx, *id)),
        ViewSwitchMessage::Back => panic!("improper use of make_view (cannot make Back)"),
    }
}

impl Library {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let switcher_model = cx.new_model(|_| {
                let mut deque = VecDeque::new();
                deque.push_back(ViewSwitchMessage::Albums);
                deque
            });
            let view = LibraryView::Album(AlbumView::new(cx, switcher_model.clone()));

            cx.subscribe(
                &switcher_model,
                move |this: &mut Library, m, message, cx| {
                    this.view = match message {
                        ViewSwitchMessage::Back => {
                            let last = m.update(cx, |v, cx| {
                                if v.len() > 1 {
                                    v.pop_back();
                                    cx.notify();

                                    v.back().cloned()
                                } else {
                                    None
                                }
                            });

                            if let Some(message) = last {
                                debug!("{:?}", message);
                                make_view(&message, cx, m)
                            } else {
                                this.view.clone()
                            }
                        }
                        _ => {
                            m.update(cx, |v, cx| {
                                if v.len() > 99 {
                                    v.pop_front();
                                }
                                v.push_back(*message);

                                cx.notify();
                            });

                            make_view(message, cx, m)
                        }
                    };

                    cx.notify();
                },
            )
            .detach();

            Library {
                navigation_view: NavigationView::new(cx, switcher_model.clone()),
                view,
            }
        })
    }
}

impl Render for Library {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .flex_shrink()
            .overflow_x_hidden()
            .child(self.navigation_view.clone())
            .child(match &self.view {
                LibraryView::Album(album_view) => album_view.clone().into_any_element(),
                LibraryView::Release(release_view) => release_view.clone().into_any_element(),
            })
    }
}
