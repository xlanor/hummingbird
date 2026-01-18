use gpui::{prelude::FluentBuilder, *};
use smallvec::SmallVec;

use crate::ui::{
    components::icons::{CROSS, MAXIMIZE, MINUS, icon},
    constants::APP_ROUNDING,
    theme::Theme,
};

#[derive(IntoElement)]
pub struct WindowHeader {
    title: Option<AnyElement>,
    left: SmallVec<[AnyElement; 2]>,
    right: SmallVec<[AnyElement; 2]>,
    div: Div,
}

impl WindowHeader {
    pub fn new() -> Self {
        Self {
            title: None,
            left: SmallVec::new(),
            right: SmallVec::new(),
            div: div(),
        }
    }

    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    pub fn left(mut self, element: impl IntoElement) -> Self {
        self.left.push(element.into_any_element());
        self
    }

    pub fn right(mut self, element: impl IntoElement) -> Self {
        self.right.push(element.into_any_element());
        self
    }
}

impl Styled for WindowHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl RenderOnce for WindowHeader {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let decorations = window.window_decorations();
        let theme = cx.global::<Theme>();

        let left_container = div()
            .pl(px(12.0))
            .pb(px(8.0))
            .pt(px(7.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .when_some(self.title, |this, title| {
                this.child(div().id("window-title").child(title))
            })
            .children(self.left);

        let right_container = div()
            .ml_auto()
            .flex()
            .items_center()
            .gap(px(8.0))
            .children(self.right);

        self.div
            .flex()
            .items_center()
            .w_full()
            .text_sm()
            .min_h(px(37.0))
            .max_h(px(37.0))
            .bg(theme.background_secondary)
            .border_b_1()
            .id("titlebar")
            .border_color(theme.border_color)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(not(target_os = "windows")), |this| {
                this.on_mouse_down(MouseButton::Left, move |ev, window, _| {
                    if ev.click_count != 2 {
                        window.start_window_move();
                    }
                })
                .on_click(|ev, window, _| {
                    if ev.click_count() == 2 {
                        window.zoom_window();
                    }
                })
            })
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling } => div
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(APP_ROUNDING)
                    })
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(APP_ROUNDING)
                    }),
            })
            .when(cfg!(target_os = "macos"), |this| {
                this.child(div().w(px(72.0)))
            })
            .child(left_container)
            .child(right_container)
            .when(cfg!(not(target_os = "macos")), |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .child(WindowButton::Minimize)
                        .child(WindowButton::Maximize)
                        .child(WindowButton::Close),
                )
            })
    }
}

pub fn header() -> WindowHeader {
    WindowHeader::new()
}

#[derive(PartialEq, Clone, Copy, IntoElement)]
pub enum WindowButton {
    Close,
    Minimize,
    Maximize,
}

impl RenderOnce for WindowButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let (bg, hover, active) = if self == WindowButton::Close {
            (
                theme.close_button,
                theme.close_button_hover,
                theme.close_button_active,
            )
        } else {
            (
                theme.window_button,
                theme.window_button_hover,
                theme.window_button_active,
            )
        };

        div()
            .flex()
            .w(px(36.0))
            .h(px(37.0))
            .pb(px(1.0))
            .items_center()
            .justify_center()
            .cursor_pointer()
            .id(match self {
                WindowButton::Close => "close",
                WindowButton::Minimize => "minimize",
                WindowButton::Maximize => "maximize",
            })
            .bg(bg)
            .hover(|this| this.bg(hover))
            .active(|this| this.bg(active))
            .window_control_area(match self {
                WindowButton::Close => WindowControlArea::Close,
                WindowButton::Minimize => WindowControlArea::Min,
                WindowButton::Maximize => WindowControlArea::Max,
            })
            .text_size(px(11.0))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .child(
                icon(match self {
                    WindowButton::Close => CROSS,
                    WindowButton::Minimize => MINUS,
                    WindowButton::Maximize => MAXIMIZE,
                })
                .size(px(14.0)),
            )
            .when(self == WindowButton::Close, |this| this.rounded_tr(px(4.0)))
            .on_click(move |_, window, cx| match self {
                WindowButton::Close => cx.quit(),
                WindowButton::Minimize => window.minimize_window(),
                WindowButton::Maximize => window.zoom_window(),
            })
    }
}
