use gpui::{prelude::FluentBuilder, *};

use crate::ui::theme::Theme;

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
                .rounded(px(4.0))
                .flex()
                .gap(px(5.0))
                .p(px(5.0))
                .w_full()
                .child(
                    div()
                        .w(px(12.0))
                        .h(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when_some(icon, |this, icon| this.child(icon)),
                )
                .child(div().child(name))
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
        div: div().w(px(150.0)),
    }
}
