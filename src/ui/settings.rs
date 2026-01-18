use gpui::{
    App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowKind, WindowOptions, div, px,
};

use crate::{
    settings::storage::DEFAULT_SIDEBAR_WIDTH,
    ui::{
        components::{
            icons::{BOOKS, PLAY},
            sidebar::{sidebar, sidebar_item},
            window_chrome::window_chrome,
            window_header::header,
        },
        theme::Theme,
    },
};

pub fn open_settings_window(cx: &mut App) {
    let bounds = WindowBounds::Windowed(gpui::Bounds::centered(
        None,
        gpui::size(px(900.0), px(600.0)),
        cx,
    ));

    cx.open_window(
        WindowOptions {
            window_bounds: Some(bounds),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            window_min_size: Some(gpui::size(px(640.0), px(420.0))),
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("Settings")),
                appears_transparent: true,
                traffic_light_position: Some(gpui::Point {
                    x: px(12.0),
                    y: px(11.0),
                }),
            }),
            kind: WindowKind::Normal,
            ..Default::default()
        },
        |window, cx| {
            window.set_window_title("Settings");
            SettingsWindow::new(cx)
        },
    )
    .ok();
}

struct SettingsWindow;

impl SettingsWindow {
    fn new(cx: &mut App) -> gpui::Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        window_chrome(
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(header().title(div().child(div().flex_grow())))
                // sidebar
                .child(
                    sidebar()
                        .width(DEFAULT_SIDEBAR_WIDTH)
                        .h_full()
                        .pt(px(8.0))
                        .pb(px(8.0))
                        .pl(px(8.0))
                        .pr(px(7.0))
                        .border_r_1()
                        .border_color(theme.border_color)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(sidebar_item("library").icon(BOOKS).child("Library"))
                        .child(sidebar_item("playback").icon(PLAY).child("Playback")),
                ),
        )
    }
}
