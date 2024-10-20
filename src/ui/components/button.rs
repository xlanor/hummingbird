use std::ops::Deref;

use gpui::*;
use prelude::FluentBuilder;
use smallvec::SmallVec;

use super::styling::AdditionalStyleUtil;

#[derive(Clone, Copy)]
enum ButtonSize {
    Regular,
    Large,
}

#[derive(Clone, Copy)]
enum ButtonIntent {
    Primary,
    Secondary,
    Warning,
    Danger,
}

#[derive(Clone, Copy)]
enum ButtonStyle {
    Regular,
    Minimal,
}

impl ButtonStyle {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonStyle::Regular => dest.border_1().cursor_pointer(),
            ButtonStyle::Minimal => dest.background_opacity(1.0).cursor_pointer(),
        }
    }

    fn hover<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonStyle::Regular => dest,
            ButtonStyle::Minimal => dest.background_opacity(0.5),
        }
    }

    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonStyle::Regular => dest,
            ButtonStyle::Minimal => dest.background_opacity(0.5),
        }
    }
}

impl ButtonSize {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonSize::Regular => dest
                .px(px(12.0))
                .py(px(8.0))
                .text_sm()
                .rounded(px(4.0))
                .gap(px(6.0)),
            ButtonSize::Large => dest
                .px(px(16.0))
                .py(px(12.0))
                .text_base()
                .rounded(px(4.0))
                .gap(px(6.0)),
        }
    }
}

impl ButtonIntent {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest.bg(rgb(0x1f2937)).border_color(rgb(0x374151)),
            ButtonIntent::Secondary => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Warning => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Danger => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
        }
    }
    fn hover<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest.bg(rgb(0x1f2937)).border_color(rgb(0x374151)),
            ButtonIntent::Secondary => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Warning => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Danger => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
        }
    }
    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest.bg(rgb(0x1f2937)).border_color(rgb(0x374151)),
            ButtonIntent::Secondary => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Warning => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
            ButtonIntent::Danger => dest.bg(rgb(0x1e293b)).text_color(rgb(0xFFFFFF)),
        }
    }
}

#[derive(IntoElement)]
pub struct Button {
    pub(self) div: Div,
    pub(self) style: ButtonStyle,
    pub(self) size: ButtonSize,
    pub(self) intent: ButtonIntent,
}

impl Button {
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn id(self, id: impl Into<ElementId>) -> InteractiveButton {
        InteractiveButton {
            div: div().id(id),
            size: self.size,
            style: self.style,
            intent: self.intent,
        }
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let style = self.style;
        let size = self.size;
        let intent = self.intent;

        style.base(size.base(intent.base(self.div.hover(|v| style.hover(intent.hover(v))))))
    }
}

#[derive(IntoElement)]
pub struct InteractiveButton {
    pub(self) div: Stateful<Div>,
    pub(self) style: ButtonStyle,
    pub(self) size: ButtonSize,
    pub(self) intent: ButtonIntent,
}

impl InteractiveButton {
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn on_click(mut self, fun: impl Fn(&ClickEvent, &mut WindowContext) + 'static) -> Self {
        self.div = self.div.on_click(fun);
        self
    }
}

impl ParentElement for InteractiveButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for InteractiveButton {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let style = self.style;
        let size = self.size;
        let intent = self.intent;

        style.base(
            size.base(
                intent.base(
                    self.div
                        .hover(|v| style.hover(intent.hover(v)))
                        .active(|v| style.active(intent.active(v))),
                ),
            ),
        )
    }
}

pub fn button() -> Button {
    Button {
        div: div(),
        style: ButtonStyle::Regular,
        size: ButtonSize::Regular,
        intent: ButtonIntent::Secondary,
    }
}
