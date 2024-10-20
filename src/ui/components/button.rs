use std::ops::Deref;

use gpui::*;
use prelude::FluentBuilder;
use smallvec::SmallVec;

use super::styling::AdditionalStyleUtil;

#[derive(Clone, Copy)]
pub enum ButtonSize {
    Regular,
    Large,
}

#[derive(Clone, Copy)]
pub enum ButtonIntent {
    Primary,
    Secondary,
    Warning,
    Danger,
}

#[derive(Clone, Copy)]
pub enum ButtonStyle {
    Regular,
    Minimal,
}

impl ButtonStyle {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        let div = dest.cursor_pointer().flex();

        match self {
            ButtonStyle::Regular => div.border_1().shadow_sm(),
            ButtonStyle::Minimal => div.background_opacity(1.0),
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
                .px(px(10.0))
                .py(px(3.0))
                .text_sm()
                .rounded(px(4.0))
                .gap(px(8.0)),
            ButtonSize::Large => dest
                .px(px(14.0))
                .py(px(5.0))
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .rounded(px(4.0))
                .gap(px(10.0)),
        }
    }

    // i have no idea what this is about but the text size changes when you click on it unless we
    // have this
    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonSize::Regular => dest.text_sm(),
            ButtonSize::Large => dest.text_sm(),
        }
    }
}

impl ButtonIntent {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest
                .bg(rgb(0x1e3a8a))
                .border_color(rgb(0x1e40af))
                .text_color(rgb(0xbfdbfe)),
            ButtonIntent::Secondary => dest.bg(rgb(0x1f2937)).border_color(rgb(0x374151)),
            ButtonIntent::Warning => dest
                .bg(rgb(0x854d0e))
                .border_color(rgb(0xa16207))
                .text_color(rgb(0xfef9c3)),
            ButtonIntent::Danger => dest
                .bg(rgb(0x7f1d1d))
                .border_color(rgb(0x991b1b))
                .text_color(rgb(0xfecaca)),
        }
    }
    fn hover<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest.bg(rgb(0x1e40af)),
            ButtonIntent::Secondary => dest.bg(rgb(0x334155)),
            ButtonIntent::Warning => dest.bg(rgb(0xa16207)),
            ButtonIntent::Danger => dest.bg(rgb(0x991b1b)),
        }
    }
    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonIntent::Primary => dest.bg(rgb(0x172554)),
            ButtonIntent::Secondary => dest.bg(rgb(0x0f172a)),
            ButtonIntent::Warning => dest.bg(rgb(0x713f12)),
            ButtonIntent::Danger => dest.bg(rgb(0x450a0a)),
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

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
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

impl Styled for InteractiveButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
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
                        .active(|v| style.active(size.active(intent.active(v)))),
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
