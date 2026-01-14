use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use smallvec::SmallVec;

use crate::ui::theme::Theme;

type ClickEvHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub struct Label {
    id: ElementId,
    text: SharedString,
    subtext: Option<SharedString>,
    group: Option<SharedString>,
    vertical: bool,
    on_click: Option<ClickEvHandler>,
    children: SmallVec<[AnyElement; 2]>,
    div: Div,
}

impl Label {
    fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    fn subtext(mut self, subtext: impl Into<SharedString>) -> Self {
        self.subtext = Some(subtext.into());
        self
    }

    fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    fn on_click(mut self, on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }
}

impl Styled for Label {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for Label {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        self.div
            .id(self.id)
            .flex()
            .gap(px(4.0))
            .when_else(
                self.vertical,
                |this| this.flex_row(),
                |this| this.flex_col(),
            )
            .child(div().child(self.text))
            .when_some(self.subtext, |this, that| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child(that)
                        .when(self.vertical, |this| this.my_auto()),
                )
            })
            .when_some(self.on_click, |this, on_click| this.on_click(on_click))
            .children(self.children)
    }
}

pub fn label(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Label {
    Label {
        id: id.into(),
        text: text.into(),
        subtext: None,
        group: None,
        children: SmallVec::new(),
        on_click: None,
        vertical: false,
        div: div(),
    }
}
