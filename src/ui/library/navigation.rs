use gpui::*;
use tracing::debug;

use crate::{
    library::db::{AlbumMethod, LibraryAccess},
    ui::components::{
        icons::{ARROW_LEFT, ARROW_RIGHT},
        nav_button::nav_button,
    },
};

use super::{NavigationHistory, ViewSwitchMessage};

pub(super) struct NavigationView {
    view_switcher_model: Entity<NavigationHistory>,
    current_message: ViewSwitchMessage,
    description: Option<SharedString>,
}

impl NavigationView {
    pub(super) fn new(
        cx: &mut App,
        view_switcher_model: Entity<NavigationHistory>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let current_message = view_switcher_model.read(cx).current();

            cx.observe(&view_switcher_model, |this: &mut NavigationView, m, cx| {
                debug!("{:#?}", m.read(cx));

                this.current_message = m.read(cx).current();

                this.description = match this.current_message {
                    ViewSwitchMessage::Release(id) => cx
                        .get_album_by_id(id, AlbumMethod::Metadata)
                        .ok()
                        .map(|v| SharedString::from(v.title.clone())),
                    _ => None,
                }
            })
            .detach();

            Self {
                view_switcher_model,
                current_message,
                description: None,
            }
        })
    }
}

impl Render for NavigationView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_go_back = self.view_switcher_model.read(cx).can_go_back();
        let can_go_forward = self.view_switcher_model.read(cx).can_go_forward();

        div().flex().child(
            div()
                .flex()
                .gap(px(4.0))
                .w_full()
                .max_w(px(1000.0))
                .mr_auto()
                .pl(px(10.0))
                .pt(px(10.0))
                .child(
                    nav_button("back", ARROW_LEFT)
                        .disabled(!can_go_back)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.view_switcher_model.update(cx, |_, cx| {
                                cx.emit(ViewSwitchMessage::Back);
                            })
                        })),
                )
                .child(
                    nav_button("forward", ARROW_RIGHT)
                        .disabled(!can_go_forward)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.view_switcher_model.update(cx, |_, cx| {
                                cx.emit(ViewSwitchMessage::Forward);
                            })
                        })),
                ),
        )
    }
}
