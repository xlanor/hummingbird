mod library;

use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, prelude::FluentBuilder, px,
};

use crate::{
    settings::storage::DEFAULT_SIDEBAR_WIDTH,
    ui::{
        components::{
            icons::{BOOKS, PLAY},
            section_header::section_header,
            sidebar::{sidebar, sidebar_item},
            window_chrome::window_chrome,
            window_header::header,
        },
        settings::library::LibrarySettings,
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

#[derive(Clone, PartialEq)]
enum SettingsSection {
    Library(Entity<LibrarySettings>),
    Playback,
}

struct SettingsWindow {
    active: SettingsSection,
}

impl SettingsWindow {
    fn new(cx: &mut App) -> gpui::Entity<Self> {
        let library = library::LibrarySettings::new(cx);
        cx.new(|_| Self {
            active: SettingsSection::Library(library),
        })
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let active = &self.active;

        let content = match active {
            SettingsSection::Library(library) => div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(library.clone())
                .into_any_element(),
            SettingsSection::Playback => section_header("Playback")
                .subtitle("Playback settings will be displayed here in the future")
                .into_any_element(),
        };

        window_chrome(
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(header().title(div().child(div().flex_grow())))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_grow()
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
                                .child(
                                    sidebar_item("library")
                                        .icon(BOOKS)
                                        .child("Library")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.active =
                                                SettingsSection::Library(LibrarySettings::new(cx));
                                            cx.notify();
                                        }))
                                        .when(
                                            matches!(active, SettingsSection::Library(_)),
                                            |this| this.active(),
                                        ),
                                )
                                .child(
                                    sidebar_item("playback")
                                        .icon(PLAY)
                                        .child("Playback")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.active = SettingsSection::Playback;
                                            cx.notify();
                                        }))
                                        .when(
                                            matches!(active, SettingsSection::Playback),
                                            |this| this.active(),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_grow()
                                .p(px(16.0))
                                .child(content),
                        ),
                ),
        )
    }
}
