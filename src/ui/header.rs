mod scrubber;

use std::sync::Arc;

use gpui::*;
use prelude::FluentBuilder;

use crate::{
    media::metadata::Metadata,
    playback::{interface::GPUIPlaybackInterface, thread::PlaybackState},
    ui::global_actions::Quit,
};

use super::{
    global_actions::{Next, PlayPause, Previous},
    models::{Models, PlaybackInfo},
};

pub struct Header {
    info_section: View<InfoSection>,
    scrubber: View<Scrubber>,
}

impl Header {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| Self {
            info_section: InfoSection::new(cx),
            scrubber: Scrubber::new(cx),
        })
    }
}

#[cfg(not(target_os = "macos"))]
impl Render for Header {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(60.0))
            .bg(rgb(0x111827))
            .border_b_1()
            .border_color(rgb(0x1e293b))
            .on_mouse_move(|_e, cx| cx.refresh())
            .on_mouse_down(MouseButton::Left, move |_, cx| cx.start_window_move())
            .flex()
            .child(self.info_section.clone())
            .child(self.scrubber.clone())
            .child(WindowControls {})
    }
}

#[cfg(target_os = "macos")]
impl Render for Header {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(60.0))
            .bg(rgb(0x111827))
            .border_b_1()
            .border_color(rgb(0x1e293b))
            .on_mouse_move(|_e, cx| cx.refresh())
            .on_mouse_down(MouseButton::Left, move |e, cx| cx.start_window_move())
            .flex()
            .child(WindowControls {})
            .child(self.info_section.clone())
            .child(self.scrubber.clone())
    }
}

pub struct InfoSection {
    metadata: Model<Metadata>,
    albumart: Model<Option<Arc<RenderImage>>>,
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

            cx.observe(&metadata_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                metadata: metadata_model,
                albumart: albumart_model,
                albumart_actual: None,
                playback_info,
            }
        })
    }
}

impl Render for InfoSection {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        cx.observe(&self.albumart, |this, m, cx| {
            let image = m.read(cx).clone();

            this.albumart_actual = image.map(ImageSource::Render);
            cx.notify()
        })
        .detach();

        let metadata = self.metadata.read(cx);
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
                            .bg(rgb(0x4b5563))
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
                                        .child(
                                            metadata
                                                .artist
                                                .clone()
                                                .unwrap_or("Unknown Artist".into()),
                                        ),
                                )
                                .child(
                                    div().overflow_x_hidden().pb(px(3.0)).text_ellipsis().child(
                                        metadata.name.clone().unwrap_or("Unknown Track".into()),
                                    ),
                                ),
                        )
                    }),
            )
    }
}

pub struct PlaybackSection {
    info: PlaybackInfo,
}

impl PlaybackSection {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let info = cx.global::<PlaybackInfo>().clone();
            let state = info.playback_state.clone();

            cx.observe(&state, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { info }
        })
    }
}

impl Render for PlaybackSection {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.info.playback_state.read(cx);

        div().absolute().flex().w_full().child(
            // TODO: position this so that it does not ever overlap with the timestamp and
            // current track info
            div()
                .mr(auto())
                .ml(auto())
                .mt(px(6.0))
                .border(px(1.0))
                .rounded(px(4.0))
                .border_color(rgb(0x374151))
                .flex()
                .child(
                    div()
                        .w(px(28.0))
                        .h(px(26.0))
                        .rounded_l(px(3.0))
                        .bg(rgb(0x1f2937))
                        .font_family("Font Awesome 6 Free")
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|style| style.bg(rgb(0x374151)).cursor_pointer())
                        .id("header-prev-button")
                        .active(|style| style.bg(rgb(0x111827)))
                        .on_mouse_down(MouseButton::Left, |_, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(|_, cx| {
                            cx.dispatch_action(Box::new(Previous));
                        })
                        .child(""),
                )
                .child(
                    div()
                        .w(px(30.0))
                        .h(px(26.0))
                        .bg(rgb(0x1f2937))
                        .border_l(px(1.0))
                        .border_r(px(1.0))
                        .border_color(rgb(0x374151))
                        .font_family("Font Awesome 6 Free")
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|style| style.bg(rgb(0x374151)).cursor_pointer())
                        .id("header-play-button")
                        .active(|style| style.bg(rgb(0x111827)))
                        .on_mouse_down(MouseButton::Left, |_, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(|_, cx| {
                            cx.dispatch_action(Box::new(PlayPause));
                        })
                        .when(*state == PlaybackState::Playing, |div| div.child(""))
                        .when(*state != PlaybackState::Playing, |div| div.child("")),
                )
                .child(
                    div()
                        .w(px(28.0))
                        .h(px(26.0))
                        .rounded_r(px(3.0))
                        .bg(rgb(0x1f2937))
                        .font_family("Font Awesome 6 Free")
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|style| style.bg(rgb(0x374151)).cursor_pointer())
                        .id("header-next-button")
                        .active(|style| style.bg(rgb(0x111827)))
                        .on_mouse_down(MouseButton::Left, |_, cx| {
                            cx.stop_propagation();
                        })
                        .on_click(|_, cx| {
                            cx.dispatch_action(Box::new(Next));
                        })
                        .child(""),
                ),
        )
    }
}

#[derive(IntoElement)]
pub struct WindowControls {}

#[cfg(target_os = "macos")]
impl RenderOnce for WindowControls {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        div().flex_shrink_0().w(px(65.0)).h_full()
    }
}

#[cfg(not(target_os = "macos"))]
impl RenderOnce for WindowControls {
    fn render(self, _: &mut WindowContext) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .font_family("Font Awesome 6 Free")
            .border_l(px(1.0))
            .border_color(rgb(0x1e293b))
            .child(
                div()
                    .flex()
                    .border_b(px(1.0))
                    .border_color(rgb(0x1e293b))
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .text_size(px(12.0))
                            .hover(|style| style.bg(rgb(0x334155)).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-minimize")
                            .on_click(|_, cx| cx.minimize_window()),
                    )
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(rgb(0x334155)).cursor_pointer())
                            .text_size(px(12.0))
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-maximize")
                            .on_click(|_, cx| cx.zoom_window()),
                    )
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex()
                            .rounded_tr(px(6.0))
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(rgb(0x991b1b)).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            })
                            .id("header-close")
                            .on_click(|_, cx| {
                                cx.dispatch_action(Box::new(Quit));
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .hover(|style| style.bg(rgb(0x334155)).cursor_pointer())
                            .text_size(px(12.0))
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            }),
                    )
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .text_size(px(12.0))
                            .hover(|style| style.bg(rgb(0x334155)).cursor_pointer())
                            .child("")
                            .on_mouse_down(MouseButton::Left, |_, cx| {
                                cx.stop_propagation();
                            }),
                    ),
            )
    }
}

pub struct Scrubbing;

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
        let position = *self.position.read(cx);
        let duration = *self.duration.read(cx);
        let remaining = duration - position;

        div()
            .pl(px(13.0))
            .pr(px(13.0))
            .border_l(px(1.0))
            .border_color(rgb(0x1e293b))
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
                    .mb(px(6.0))
                    .mt(px(6.0))
                    .child(deferred(self.playback_section.clone()))
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
                    .child(div().h(px(30.0)))
                    .child(div().ml(auto()).child(format!(
                        "-{:02}:{:02}",
                        remaining / 60,
                        remaining % 60
                    ))),
            )
            .child(
                div()
                    .w_full()
                    .h(px(6.0))
                    .bg(rgb(0x374151))
                    .rounded(px(3.0))
                    .child(div().w_full().h(px(6.0)).child(scrubber::Scrubber::new(
                        Some(ElementId::from("scrubber")),
                        duration,
                        position,
                    )))
                    .id("scrubber-back")
                    .on_mouse_down(MouseButton::Left, |_, cx| {
                        cx.stop_propagation();
                    })
                    .on_drag(Scrubbing, |_, cx| cx.new_view(|_| EmptyView))
                    .on_drag_move(move |ev: &DragMoveEvent<Scrubbing>, cx| {
                        let interface = cx.global::<GPUIPlaybackInterface>();
                        let relative = cx.mouse_position() - ev.bounds.origin;
                        let relative_x = relative.x.0;
                        let width = ev.bounds.size.width.0;
                        let position = (relative_x / width).clamp(0.0, 1.0);
                        let seconds = position as f64 * duration as f64;

                        interface.seek(seconds);
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
