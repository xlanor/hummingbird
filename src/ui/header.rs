use gpui::*;
use prelude::FluentBuilder;
use tracing::debug;

use crate::{
    data::interface::GPUIDataInterface,
    playback::{interface::GPUIPlaybackInterface, thread::PlaybackState},
    ui::global_actions::Quit,
};

use super::{
    components::slider::slider,
    constants::{APP_ROUNDING, FONT_AWESOME},
    global_actions::{Next, PlayPause, Previous},
    models::{Models, PlaybackInfo},
    theme::Theme,
};

pub struct Header {
    info_section: View<InfoSection>,
    scrubber: View<Scrubber>,
    show_queue: Model<bool>,
}

impl Header {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>, show_queue: Model<bool>) -> View<Self> {
        cx.new_view(|cx| Self {
            info_section: InfoSection::new(cx),
            scrubber: Scrubber::new(cx),
            show_queue,
        })
    }
}

#[cfg(not(target_os = "macos"))]
impl Render for Header {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let decorations = cx.window_decorations();
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h(px(60.0))
            .bg(theme.background_secondary)
            .border_b_1()
            .border_color(theme.border_color)
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
            .id("header")
            .when(cfg!(target_os = "windows"), |this| {
                this.on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
            })
            .when(cfg!(not(target_os = "windows")), |this| {
                this.on_mouse_down(MouseButton::Left, move |ev, cx| {
                    if ev.click_count != 2 {
                        cx.start_window_move();
                    }
                })
                .on_click(|ev, cx| {
                    if ev.down.click_count == 2 {
                        debug!("double clicked");
                        cx.zoom_window();
                    }
                })
            })
            .flex()
            .child(self.info_section.clone())
            .child(self.scrubber.clone())
            .child(WindowControls {
                show_queue: self.show_queue.clone(),
            })
    }
}

#[cfg(target_os = "macos")]
impl Render for Header {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h(px(60.0))
            .bg(theme.background_primary)
            .border_b_1()
            .border_color(theme.border_color)
            // macOS doesn't ever actually stop rounding corners so we don't need to check for
            // tiling
            .rounded_t(APP_ROUNDING)
            .id("header")
            .on_mouse_down(MouseButton::Left, move |e, cx| cx.start_window_move())
            .on_click(|ev, cx| {
                if ev.down.click_count == 2 {
                    cx.zoom_window();
                }
            })
            .flex()
            .child(div().flex_shrink_0().w(px(67.0)).h_full())
            .child(self.info_section.clone())
            .child(self.scrubber.clone())
            .child(WindowControls {
                show_queue: self.show_queue.clone(),
            })
    }
}

pub struct InfoSection {
    track_name: Option<SharedString>,
    artist_name: Option<SharedString>,
    albumart_actual: Option<ImageSource>,
    playback_info: PlaybackInfo,
}

impl InfoSection {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let metadata_model = cx.global::<Models>().metadata.clone();
            let albumart_model = cx.global::<Models>().albumart.clone();
            let playback_info = cx.global::<PlaybackInfo>().clone();

            cx.observe(&playback_info.playback_state, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&metadata_model, |this: &mut Self, m, cx| {
                let metadata = m.read(cx);

                this.track_name = metadata.name.clone().map(|v| SharedString::from(v));
                this.artist_name = metadata.artist.clone().map(|v| SharedString::from(v));

                cx.notify();
            })
            .detach();

            cx.observe(&albumart_model, |this: &mut Self, m, cx| {
                let image = m.read(cx).clone();

                this.albumart_actual = image.map(ImageSource::Render);
                cx.notify()
            })
            .detach();

            Self {
                artist_name: None,
                track_name: None,
                albumart_actual: None,
                playback_info,
            }
        })
    }
}

impl Render for InfoSection {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = self.playback_info.playback_state.read(cx);

        div()
            .id("info-section")
            .flex()
            .w(px(275.0))
            .min_w(px(275.0))
            .max_w(px(275.0))
            .overflow_x_hidden()
            .flex_shrink_0()
            .child(
                div()
                    .mx(px(12.0))
                    .mt(px(12.0))
                    .mb(px(9.0))
                    .gap(px(10.0))
                    .flex()
                    .overflow_x_hidden()
                    .child(
                        div()
                            .id("album-art")
                            .rounded(px(4.0))
                            .bg(theme.album_art_background)
                            .shadow_sm()
                            .w(px(36.0))
                            .h(px(36.0))
                            .when(self.albumart_actual.is_some(), |div| {
                                div.child(
                                    img(self.albumart_actual.clone().unwrap())
                                        .w(px(36.0))
                                        .h(px(36.0))
                                        .rounded(px(4.0)),
                                )
                            }),
                    )
                    .when(*state == PlaybackState::Stopped, |e| {
                        e.child(
                            div()
                                .line_height(rems(1.0))
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .text_size(px(15.0))
                                .flex()
                                .h_full()
                                .items_center()
                                .pb(px(3.0))
                                .child("Muzak"),
                        )
                    })
                    .when(*state != PlaybackState::Stopped, |e| {
                        e.child(
                            div()
                                .flex()
                                .flex_col()
                                .line_height(rems(1.0))
                                .text_size(px(15.0))
                                .gap_1()
                                .overflow_x_hidden()
                                .child(
                                    div()
                                        .overflow_x_hidden()
                                        .font_weight(FontWeight::EXTRA_BOLD)
                                        .text_ellipsis()
                                        .child(
                                            self.track_name
                                                .clone()
                                                .unwrap_or("Unknown Track".into()),
                                        ),
                                )
                                .child(div().overflow_x_hidden().pb(px(3.0)).child(
                                    self.artist_name.clone().unwrap_or("Unknown Artist".into()),
                                )),
                        )
                    }),
            )
    }
}

pub struct PlaybackSection {
    info: PlaybackInfo,
    show_volume: Model<bool>,
}

impl PlaybackSection {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let info = cx.global::<PlaybackInfo>().clone();
            let state = info.playback_state.clone();
            let volume = info.volume.clone();
            let shuffling = info.shuffling.clone();
            let show_volume: Model<bool> = cx.new_model(|_| false);

            cx.observe(&state, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&volume, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&shuffling, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&show_volume, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { info, show_volume }
        })
    }
}

impl Render for PlaybackSection {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.info.playback_state.read(cx);
        let volume = self.info.volume.read(cx);
        let shuffling = self.info.shuffling.read(cx);
        let theme = cx.global::<Theme>();
        let show_volume = self.show_volume.read(cx);
        let show_volume_model = self.show_volume.clone();

        div()
            .mr(auto())
            .ml(auto())
            .mt(px(5.0))
            .flex()
            .w_full()
            .absolute()
            .child(
                div()
                    .rounded(px(3.0))
                    .w(px(28.0))
                    .h(px(25.0))
                    .mt(px(2.0))
                    .mr(px(6.0))
                    .ml_auto()
                    .border_color(theme.playback_button_border)
                    .font_family(FONT_AWESOME)
                    .text_size(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                    .id("header-shuffle-button")
                    .active(|style| style.bg(theme.playback_button_active))
                    .on_mouse_down(MouseButton::Left, |_, cx| {
                        cx.stop_propagation();
                        cx.prevent_default();
                    })
                    .on_click(|_, cx| {
                        cx.global::<GPUIPlaybackInterface>().toggle_shuffle();
                    })
                    .when(*shuffling, |this| this.child(""))
                    .when(!shuffling, |this| this.child("")),
            )
            .child(
                div()
                    .rounded(px(4.0))
                    .border_color(theme.playback_button_border)
                    .border_1()
                    .flex()
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(28.0))
                            .rounded_l(px(3.0))
                            .bg(theme.playback_button)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-prev-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                                cx.prevent_default();
                            })
                            .on_click(|_, cx| {
                                cx.dispatch_action(Box::new(Previous));
                            })
                            .child(""),
                    )
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(28.0))
                            .bg(theme.playback_button)
                            .border_l(px(1.0))
                            .border_r(px(1.0))
                            .border_color(theme.playback_button_border)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-play-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                                cx.prevent_default();
                            })
                            .on_click(|_, cx| {
                                cx.dispatch_action(Box::new(PlayPause));
                            })
                            .when(*state == PlaybackState::Playing, |div| div.child(""))
                            .when(*state != PlaybackState::Playing, |div| div.child("")),
                    )
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(28.0))
                            .rounded_r(px(3.0))
                            .bg(theme.playback_button)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-next-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                                cx.prevent_default();
                            })
                            .on_click(|_, cx| {
                                cx.dispatch_action(Box::new(Next));
                            })
                            .child(""),
                    ),
            )
            .child(
                div()
                    .rounded(px(3.0))
                    .w(px(28.0))
                    .h(px(25.0))
                    .mt(px(2.0))
                    .ml(px(6.0))
                    .border_color(theme.playback_button_border)
                    .font_family(FONT_AWESOME)
                    .text_size(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!(*show_volume), |div| div.mr_auto())
                    .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                    .id("header-shuffle-button")
                    .active(|style| style.bg(theme.playback_button_active))
                    .on_mouse_down(MouseButton::Left, |_, cx| {
                        cx.stop_propagation();
                        cx.prevent_default();
                    })
                    .on_click(move |_, cx| {
                        show_volume_model.update(cx, |a, cx| {
                            *a = !(*a);
                            cx.notify();
                        })
                    })
                    .child(""),
            )
            .when(*show_volume, |div| {
                div.child(
                    slider()
                        .w(px(80.0))
                        .h(px(6.0))
                        .mt(px(11.0))
                        .ml(px(10.0))
                        .mr_auto()
                        .rounded(px(3.0))
                        .id("volume")
                        .value((*volume) as f32)
                        .on_change(move |v, cx| {
                            cx.global::<GPUIPlaybackInterface>().set_volume(v as f64);
                        }),
                )
            })
    }
}

#[derive(IntoElement)]
pub struct WindowControls {
    pub show_queue: Model<bool>,
}

#[cfg(target_os = "macos")]
impl RenderOnce for WindowControls {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .font_family(FONT_AWESOME)
            .border_l(px(1.0))
            .border_color(theme.border_color)
            .child(
                // FIXME: These buttons are a weird size because they need to be about the same
                // size as the buttons in Zed right now
                // Once GPUI adds support for setting the button size on Windows, set this to
                // 30x30
                div()
                    .w(px(32.0))
                    .h(px(30.0))
                    .flex()
                    .bg(theme.window_button)
                    .items_center()
                    .justify_center()
                    .flex_shrink_0()
                    .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                    .text_size(px(12.0))
                    .child("")
                    .on_mouse_down(MouseButton::Left, |_, cx| {
                        cx.stop_propagation();
                    })
                    .id("show-queue")
                    .on_click(move |_, cx| self.show_queue.update(cx, |v, _| *v = !(*v)))
                    .active(|style| style.bg(theme.window_button_active)),
            )
            .child(
                div()
                    .w(px(32.0))
                    .h(px(30.0))
                    .flex()
                    .items_center()
                    .bg(theme.window_button)
                    .justify_center()
                    .flex_shrink_0()
                    .text_size(px(12.0))
                    .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                    .child("")
                    .on_mouse_down(MouseButton::Left, |_, cx| {
                        cx.stop_propagation();
                    })
                    .id("settings")
                    .active(|style| style.bg(theme.window_button_active)),
            )
    }
}

#[cfg(not(target_os = "macos"))]
impl RenderOnce for WindowControls {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let decorations = cx.window_decorations();
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .flex_col()
            .font_family(FONT_AWESOME)
            .border_l(px(1.0))
            .border_color(theme.border_color)
            .child(
                // FIXME: These buttons are a weird size because they need to be about the same
                // size as the buttons in Zed right now
                // Once GPUI adds support for setting the button size on Windows, set this to
                // 30x30
                div()
                    .flex()
                    .border_b(px(1.0))
                    .border_color(theme.border_color)
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(30.0))
                            .flex()
                            .bg(theme.window_button)
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .text_size(px(12.0))
                            .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-minimize")
                            .on_click(|_, cx| cx.minimize_window())
                            .active(|style| style.bg(theme.window_button_active)),
                    )
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(30.0))
                            .flex()
                            .bg(theme.window_button)
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                            .text_size(px(12.0))
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-maximize")
                            .on_click(|_, cx| cx.zoom_window())
                            .active(|style| style.bg(theme.window_button_active)),
                    )
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(30.0))
                            .flex()
                            .bg(theme.close_button)
                            .map(|div| match decorations {
                                Decorations::Server => div,
                                Decorations::Client { tiling } => div
                                    .when(!(tiling.top || tiling.right), |div| {
                                        div.rounded_tr(APP_ROUNDING)
                                    }),
                            })
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(theme.close_button_hover).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-close")
                            .on_click(|_, cx| {
                                cx.dispatch_action(Box::new(Quit));
                            })
                            .active(|style| style.bg(theme.close_button_active)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(30.0))
                            .flex()
                            .bg(theme.window_button)
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                            .text_size(px(12.0))
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("show-queue")
                            .on_click(move |_, cx| self.show_queue.update(cx, |v, _| *v = !(*v)))
                            .active(|style| style.bg(theme.window_button_active)),
                    )
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(30.0))
                            .flex()
                            .bg(theme.window_button)
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .text_size(px(12.0))
                            .hover(|style| style.bg(theme.window_button_hover).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("settings")
                            .active(|style| style.bg(theme.window_button_active)),
                    ),
            )
    }
}

pub struct Scrubber {
    position: Model<u64>,
    duration: Model<u64>,
    playback_section: View<PlaybackSection>,
}

impl Scrubber {
    fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let position_model = cx.global::<PlaybackInfo>().position.clone();
            let duration_model = cx.global::<PlaybackInfo>().duration.clone();

            cx.observe(&position_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&duration_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                position: position_model,
                duration: duration_model,
                playback_section: PlaybackSection::new(cx),
            }
        })
    }
}

impl Render for Scrubber {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let position = *self.position.read(cx);
        let duration = *self.duration.read(cx);
        let remaining = duration - position;

        div()
            .pl(px(13.0))
            .pr(px(13.0))
            .border_l(px(1.0))
            .border_color(theme.border_color)
            .flex_grow()
            .flex()
            .flex_col()
            .line_height(rems(1.0))
            .text_size(px(15.0))
            .font_family("CommitMono")
            .font_weight(FontWeight::BOLD)
            .child(
                div()
                    .w_full()
                    .flex()
                    .relative()
                    .items_end()
                    .mt(px(6.0))
                    .mb(px(6.0))
                    .child(
                        div()
                            .pr(px(6.0))
                            .border_r(px(2.0))
                            .border_color(rgb(0x4b5563))
                            .child(format!("{:02}:{:02}", position / 60, position % 60)),
                    )
                    .child(div().ml(px(6.0)).text_color(rgb(0xcbd5e1)).child(format!(
                        "{:02}:{:02}",
                        duration / 60,
                        duration % 60
                    )))
                    .child(deferred(self.playback_section.clone()))
                    .child(div().h(px(30.0)))
                    .child(div().ml(auto()).child(format!(
                        "-{:02}:{:02}",
                        remaining / 60,
                        remaining % 60
                    ))),
            )
            .child(
                slider()
                    .w_full()
                    .h(px(6.0))
                    .rounded(px(3.0))
                    .id("scrubber-back")
                    .value(position as f32 / duration as f32)
                    .on_change(move |v, cx| {
                        if duration > 0 {
                            cx.global::<GPUIPlaybackInterface>()
                                .seek(v as f64 * duration as f64);
                        }
                    }),
            )
    }
}

pub struct EmptyView;

impl Render for EmptyView {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
    }
}
