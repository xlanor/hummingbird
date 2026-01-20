use gpui::{
    App, Div, FontWeight, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use crate::ui::{components::icons::icon, theme::Theme};

pub struct Callout {
    pub title: Option<SharedString>,
    pub caption: SharedString,
    pub icon: Option<&'static str>,
    pub child_div: Div,
}

impl Callout {
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl RenderOnce for Callout {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .bg(theme.callout_background)
            .text_color(theme.callout_text)
            .flex()
            .gap(px(8.0))
            .when_some(self.icon, |this, the_icon| {
                this.child(icon(the_icon).size(px(16.0)))
            })
            .child(
                div()
                    .flex_col()
                    .gap(px(8.0))
                    .when_some(self.title, |this, title| {
                        this.child(div().font_weight(FontWeight::BOLD).child(title))
                    })
                    .child(div().text_sm().child(self.caption))
                    .child(self.child_div),
            )
    }
}

pub fn callout(caption: impl Into<SharedString>) -> Callout {
    Callout {
        title: None,
        caption: caption.into(),
        icon: None,
        child_div: div(),
    }
}
