use gpui::*;
use prelude::FluentBuilder;

use crate::ui::theme::Theme;

type OnExitHandler = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App)>;

pub struct Modal {
    div: Stateful<Div>,
    on_exit: Option<OnExitHandler>,
}

impl Modal {
    fn new() -> Modal {
        Modal {
            div: div().id("modal-fg"),
            on_exit: None,
        }
    }

    pub fn on_exit(mut self, handler: OnExitHandler) -> Self {
        self.on_exit = Some(handler);
        self
    }
}

impl ParentElement for Modal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for Modal {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        anchored().position(point(px(0.0), px(0.0))).child(
            div()
                .flex()
                .w_full()
                .h_full()
                .bg(theme.modal_overlay_bg)
                .id("modal-bg")
                .when_some(self.on_exit, |this, on_exit| {
                    this.on_any_mouse_down(on_exit)
                })
                .child(
                    self.div
                        .m_auto()
                        .border_color(theme.border_color)
                        .bg(theme.background_primary)
                        .rounded(px(8.0))
                        .flex_col()
                        .on_any_mouse_down(|_, _, cx| {
                            cx.stop_propagation();
                        }),
                ),
        )
    }
}

pub fn modal() -> Modal {
    Modal::new()
}
