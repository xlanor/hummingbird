use core::panic;
use std::fs;

use gpui::*;
use prelude::FluentBuilder;
use sqlx::SqlitePool;
use tracing::{debug, error};

use crate::{
    data::{interface::GPUIDataInterface, thread::DataThread},
    library::{
        db::{create_cache, create_pool},
        scan::{ScanInterface, ScanThread},
    },
    playback::{interface::GPUIPlaybackInterface, thread::PlaybackThread},
};

use super::{
    arguments::parse_args_and_prepare, assets::Assets, constants::APP_ROUNDING,
    global_actions::register_actions, header::Header, library::Library, models::build_models,
    queue::Queue, statusbar::StatusBar,
};

struct WindowShadow {
    pub header: View<Header>,
    pub queue: View<Queue>,
    pub library: View<Library>,
    pub status_bar: View<StatusBar>,
}

impl Render for WindowShadow {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let decorations = cx.window_decorations();
        let rounding = APP_ROUNDING;
        let shadow_size = px(10.0);
        let border_size = px(1.0);
        cx.set_client_inset(shadow_size);

        div()
            .id("window-backdrop")
            .key_context("app")
            .bg(transparent_black())
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling, .. } => div
                    .bg(gpui::transparent_black())
                    .child(
                        canvas(
                            |_bounds, cx| {
                                cx.insert_hitbox(
                                    Bounds::new(
                                        point(px(0.0), px(0.0)),
                                        cx.window_bounds().get_bounds().size,
                                    ),
                                    false,
                                )
                            },
                            move |_bounds, hitbox, cx| {
                                let mouse = cx.mouse_position();
                                let size = cx.window_bounds().get_bounds().size;
                                let Some(edge) = resize_edge(mouse, shadow_size, size, tiling)
                                else {
                                    return;
                                };
                                cx.set_cursor_style(
                                    match edge {
                                        ResizeEdge::Top | ResizeEdge::Bottom => {
                                            CursorStyle::ResizeUpDown
                                        }
                                        ResizeEdge::Left | ResizeEdge::Right => {
                                            CursorStyle::ResizeLeftRight
                                        }
                                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                            CursorStyle::ResizeUpLeftDownRight
                                        }
                                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                            CursorStyle::ResizeUpRightDownLeft
                                        }
                                    },
                                    &hitbox,
                                );
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(rounding)
                    })
                    .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                    .when(!(tiling.bottom || tiling.right), |div| {
                        div.rounded_br(rounding)
                    })
                    .when(!(tiling.bottom || tiling.left), |div| {
                        div.rounded_bl(rounding)
                    })
                    .when(!tiling.top, |div| div.pt(shadow_size))
                    .when(!tiling.bottom, |div| div.pb(shadow_size))
                    .when(!tiling.left, |div| div.pl(shadow_size))
                    .when(!tiling.right, |div| div.pr(shadow_size))
                    .on_mouse_down(MouseButton::Left, move |e, cx| {
                        let size = cx.window_bounds().get_bounds().size;
                        let pos = e.position;

                        if let Some(edge) = resize_edge(pos, shadow_size, size, tiling) {
                            cx.start_window_resize(edge)
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .font_family("Inter")
                    .text_color(rgb(0xf1f5f9))
                    .cursor(CursorStyle::Arrow)
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .border_color(rgba(0x64748b33))
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(rounding)
                            })
                            .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                            .when(!(tiling.bottom || tiling.right), |div| {
                                div.rounded_br(rounding)
                            })
                            .when(!(tiling.bottom || tiling.left), |div| {
                                div.rounded_bl(rounding)
                            })
                            .when(!tiling.top, |div| div.border_t(border_size))
                            .when(!tiling.bottom, |div| div.border_b(border_size))
                            .when(!tiling.left, |div| div.border_l(border_size))
                            .when(!tiling.right, |div| div.border_r(border_size))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(smallvec::smallvec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.4,
                                    },
                                    blur_radius: shadow_size / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, cx| {
                        cx.stop_propagation();
                    })
                    .overflow_hidden()
                    .bg(gpui::rgb(0x030712))
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(self.header.clone())
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .relative()
                            .max_w_full()
                            .max_h_full()
                            .overflow_hidden()
                            .child(self.library.clone())
                            .child(self.queue.clone()),
                    )
                    .child(self.status_bar.clone()),
            )
    }
}

fn resize_edge(
    pos: Point<Pixels>,
    shadow_size: Pixels,
    size: Size<Pixels>,
    tiling: Tiling,
) -> Option<ResizeEdge> {
    let edge = if pos.y < shadow_size && pos.x < shadow_size && !tiling.top && !tiling.left {
        ResizeEdge::TopLeft
    } else if pos.y < shadow_size
        && pos.x > size.width - shadow_size
        && !tiling.top
        && !tiling.right
    {
        ResizeEdge::TopRight
    } else if pos.y < shadow_size && !tiling.top {
        ResizeEdge::Top
    } else if pos.y > size.height - shadow_size
        && pos.x < shadow_size
        && !tiling.bottom
        && !tiling.left
    {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - shadow_size
        && pos.x > size.width - shadow_size
        && !tiling.bottom
        && !tiling.right
    {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - shadow_size && !tiling.bottom {
        ResizeEdge::Bottom
    } else if pos.x < shadow_size && !tiling.left {
        ResizeEdge::Left
    } else if pos.x > size.width - shadow_size && !tiling.right {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
}

pub fn find_fonts(cx: &mut AppContext) -> gpui::Result<()> {
    let paths = cx.asset_source().list("fonts")?;
    let mut fonts = vec![];
    for path in paths {
        if path.ends_with(".ttf") || path.ends_with(".otf") {
            if let Some(v) = cx.asset_source().load(&path)? {
                fonts.push(v);
            }
        }
    }

    let results = cx.text_system().add_fonts(fonts);
    debug!("loaded fonts: {:?}", cx.text_system().all_font_names());
    results
}

pub struct Pool(pub SqlitePool);

impl Global for Pool {}

pub async fn run() {
    let dirs = directories::ProjectDirs::from("me", "william341", "muzak")
        .expect("couldn't find project dirs");
    let directory = dirs.data_dir();
    if !directory.exists() {
        fs::create_dir_all(directory)
            .unwrap_or_else(|e| panic!("couldn't create data directory, {:?}, {:?}", directory, e));
    }
    let file = directory.join("library.db");

    let pool = create_pool(file).await;

    App::new().with_assets(Assets).run(|cx: &mut AppContext| {
        let bounds = Bounds::centered(None, size(px(1024.0), px(700.0)), cx);
        find_fonts(cx).expect("unable to load fonts");

        register_actions(cx);

        build_models(cx);

        if let Ok(pool) = pool {
            let mut scan_interface: ScanInterface = ScanThread::start(pool.clone());
            scan_interface.scan();
            scan_interface.start_broadcast(cx);

            cx.set_global(scan_interface);
            cx.set_global(Pool(pool));
        } else {
            error!("unable to create database pool: {}", pool.err().unwrap());
            panic!("fatal: unable to create database pool");
        }

        let mut playback_interface: GPUIPlaybackInterface = PlaybackThread::start();
        let mut data_interface: GPUIDataInterface = DataThread::start();

        playback_interface.start_broadcast(cx);
        data_interface.start_broadcast(cx);

        parse_args_and_prepare(&playback_interface, &data_interface);

        cx.set_global(playback_interface);
        cx.set_global(data_interface);
        cx.set_global(create_cache());

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_background: WindowBackgroundAppearance::Opaque,
                window_decorations: Some(WindowDecorations::Client),
                window_min_size: Some(size(px(800.0), px(600.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Muzak")),
                    // TODO: fix this
                    appears_transparent: true,
                    traffic_light_position: Some(Point {
                        x: px(12.0),
                        y: px(12.0),
                    }),
                }),
                kind: WindowKind::Normal,
                ..Default::default()
            },
            |cx| {
                cx.new_view(|cx| {
                    cx.observe_window_appearance(|_, cx| {
                        cx.refresh();
                    })
                    .detach();
                    WindowShadow {
                        header: Header::new(cx),
                        queue: Queue::new(cx),
                        library: Library::new(cx),
                        status_bar: StatusBar::new(cx),
                    }
                })
            },
        )
        .unwrap();
    });
}
