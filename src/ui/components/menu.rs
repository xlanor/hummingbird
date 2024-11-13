use gpui::{prelude::FluentBuilder, *};

use crate::ui::{constants::FONT_AWESOME, theme::Theme};

type ClickEvHandler = dyn Fn(&ClickEvent, &mut WindowContext);

// named like this to not conflict with GPUI's MenuItem
#[derive(IntoElement)]
pub enum CMenuItem {
    Item(
        ElementId,
        Option<SharedString>,
        SharedString,
        Box<ClickEvHandler>,
    ),
    Seperator,
    Header(SharedString),
}

impl RenderOnce for CMenuItem {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        match self {
            CMenuItem::Item(id, icon, name, func) => div()
                .id(id)
                .on_click(func)
                .rounded(px(3.0))
                .flex()
                .px(px(8.0))
                .pt(px(2.0))
                .pb(px(3.0))
                .min_w_full()
                .bg(theme.menu_item)
                .hover(|this| this.bg(theme.menu_item_hover))
                .active(|this| this.bg(theme.menu_item_active))
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(div().child(name))
                .child(
                    div()
                        .w(px(12.0))
                        .h(px(12.0))
                        .my_auto()
                        .ml_auto()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.0))
                        .font_family(FONT_AWESOME)
                        .text_color(theme.text_secondary)
                        .when_some(icon, |this, icon| this.child(icon)),
                )
                .into_any_element(),
            CMenuItem::Seperator => div()
                .w_full()
                .h(px(1.0))
                .bg(theme.elevated_border_color)
                .into_any_element(),
            CMenuItem::Header(_) => div().into_any_element(), // TODO: implement this
        }
    }
}

pub fn menu_item(
    id: impl Into<ElementId>,
    icon: Option<impl Into<SharedString>>,
    text: impl Into<SharedString>,
    func: impl Fn(&ClickEvent, &mut WindowContext) + 'static,
) -> CMenuItem {
    CMenuItem::Item(
        id.into(),
        icon.map(|v| v.into()),
        text.into(),
        Box::new(func),
    )
}

#[derive(IntoElement)]
pub struct Menu {
    pub(self) items: Vec<CMenuItem>,
    pub(self) div: Div,
}

impl Menu {
    pub fn item(mut self, item: CMenuItem) -> Self {
        self.items.push(item);

        self
    }
}

impl RenderOnce for Menu {
    fn render(self, _: &mut WindowContext) -> impl IntoElement {
        self.div.flex().flex_col().children(self.items)
    }
}

pub fn menu() -> Menu {
    Menu {
        items: vec![],
        div: div().min_w(px(200.0)).p(px(4.0)),
    }
}
