use gpui::{prelude::FluentBuilder, *};

use crate::ui::components::icons::{CHECK, icon};
use crate::ui::theme::Theme;

#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    checked: bool,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, checked: bool) -> Self {
        Self {
            id: id.into(),
            checked,
        }
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .id(self.id)
            .rounded(px(4.0))
            .flex()
            .w(px(24.0))
            .h(px(24.0))
            .items_center()
            .justify_center()
            .line_height(rems(1.25))
            .bg(theme.checkbox_background)
            .border_1()
            .border_color(theme.checkbox_border)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(|this| this.bg(theme.checkbox_background_active))
            .active(|this| this.bg(theme.checkbox_background_hover))
            .when(self.checked, |this| {
                this.child(
                    div()
                        .w(px(18.0))
                        .h(px(18.0))
                        .mr(px(7.0))
                        .pt(px(0.5))
                        .my_auto()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            icon(CHECK)
                                .size(px(18.0))
                                .text_color(theme.checkbox_checked),
                        ),
                )
            })
    }
}

/// Checkbox display element.
///
/// This checkbox element **does not support click handlers.** This is because the click handler
/// should be attached to a label, which should always be used when using a checkbox.
pub fn checkbox(id: impl Into<ElementId>, checked: bool) -> Checkbox {
    Checkbox::new(id, checked)
}
