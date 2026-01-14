use gpui::{
    App, AppContext, Context, Entity, FocusHandle, InteractiveElement, ParentElement, Refineable,
    Render, SharedString, StyleRefinement, Styled, Window, div, px,
};

use crate::ui::{components::input::TextInput, theme::Theme};

pub struct Textbox {
    input: Entity<TextInput>,
    handle: FocusHandle,
    style: StyleRefinement,
}

impl Textbox {
    pub fn new(cx: &mut App, style: StyleRefinement) -> Entity<Self> {
        cx.new(|cx| {
            let handle = cx.focus_handle();

            Self {
                style,
                handle: handle.clone(),
                input: TextInput::new(
                    cx,
                    handle,
                    None,
                    Some("Enter an absolute path or select a folder".into()),
                    None,
                ),
            }
        })
    }

    pub fn value(&self, cx: &App) -> SharedString {
        self.input.read(cx).content.clone()
    }
}

impl Render for Textbox {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        let mut main = div();

        main.style().refine(&self.style);

        main.track_focus(&self.handle)
            .border_1()
            .border_color(theme.textbox_border)
            .rounded(px(4.0))
            .px(px(4.0))
            .py(px(2.0))
            .bg(theme.textbox_background)
            .child(self.input.clone())
    }
}
