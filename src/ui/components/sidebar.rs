use gpui::{
    App, Div, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use crate::{
    settings::storage::DEFAULT_SIDEBAR_WIDTH,
    ui::{components::icons::icon, theme::Theme, util::MaybeStateful},
};

#[derive(IntoElement)]
pub struct Sidebar {
    div: MaybeStateful<Div>,
    width: Option<Entity<Pixels>>,
}

impl Sidebar {
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.div = MaybeStateful::Stateful(match self.div {
            MaybeStateful::NotStateful(div) => div.id(id),
            MaybeStateful::Stateful(div) => div,
        });

        self
    }

    pub fn width(mut self, width: Entity<Pixels>) -> Self {
        self.width = Some(width);
        self
    }
}

impl Styled for Sidebar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let width: Pixels = match self.width {
            Some(w) => *w.read(cx),
            None => DEFAULT_SIDEBAR_WIDTH,
        };
        self.div.w(width).flex().flex_col()
    }
}

pub fn sidebar() -> Sidebar {
    Sidebar {
        div: MaybeStateful::NotStateful(div()),
        width: None,
    }
}

#[derive(IntoElement)]
pub struct SidebarItem {
    parent_div: Stateful<Div>,
    children_div: Div,
    icon: Option<&'static str>,
    active: bool,
}

impl SidebarItem {
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }
}

impl Styled for SidebarItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.parent_div.style()
    }
}
impl ParentElement for SidebarItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.children_div.extend(elements);
    }
}

impl StatefulInteractiveElement for SidebarItem {}

impl InteractiveElement for SidebarItem {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.parent_div.interactivity()
    }
}

impl RenderOnce for SidebarItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        self.parent_div
            .flex()
            .bg(theme.background_primary)
            .text_sm()
            .when(self.active, |div| div.bg(theme.background_tertiary))
            .rounded(px(4.0))
            .px(px(9.0))
            .py(px(7.0))
            .line_height(px(18.0))
            .gap(px(6.0))
            .font_weight(FontWeight::SEMIBOLD)
            .hover(|this| this.bg(theme.nav_button_hover))
            .active(|this| this.bg(theme.nav_button_active))
            .when_none(&self.icon, |this| this.child(div().size(px(18.0))))
            .when_some(self.icon, |this, used_icon| {
                this.child(icon(used_icon).size(px(18.0)))
            })
            .child(self.children_div)
    }
}

pub fn sidebar_item(id: impl Into<ElementId>) -> SidebarItem {
    SidebarItem {
        parent_div: div().id(id),
        children_div: div(),
        icon: None,
        active: false,
    }
}

#[derive(IntoElement)]
pub struct SidebarSeparator {}

impl RenderOnce for SidebarSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .my(px(6.0))
            .border_b_1()
            .border_color(theme.border_color)
    }
}

pub fn sidebar_separator() -> SidebarSeparator {
    SidebarSeparator {}
}
