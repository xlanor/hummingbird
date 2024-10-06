use gpui::*;

use crate::{playback::interface::GPUIPlaybackInterface, ui::models::PlaybackInfo};

pub struct Scrubber {
    id: Option<ElementId>,
    duration: u64,
    position: u64,
}

impl Scrubber {
    pub fn new(id: Option<ElementId>, duration: u64, position: u64) -> Self {
        Self {
            id,
            duration,
            position,
        }
    }
}

pub struct PrepaintState {}
pub struct RequestLayoutState {}

impl IntoElement for Scrubber {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Scrubber {
    type RequestLayoutState = RequestLayoutState;

    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = px(6.0).into();
        (cx.request_layout(style, []), RequestLayoutState {})
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        PrepaintState {}
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        let progress = self.position as f32 / self.duration as f32;
        let mut inner_bounds = bounds;
        inner_bounds.size.width = bounds.size.width * progress;

        cx.paint_quad(quad(
            bounds,
            px(3.0),
            rgb(0x374151),
            Edges::all(px(0.0)),
            rgb(0x000000),
        ));

        cx.paint_quad(quad(
            inner_bounds,
            px(3.0),
            rgb(0x3b82f6),
            Edges::all(px(0.0)),
            rgb(0x000000),
        ));

        let cloned_bounds = bounds;
        let duration = self.duration;

        cx.on_mouse_event(move |ev: &MouseDownEvent, _, cx| {
            let playing = cx.global::<PlaybackInfo>().current_track.read(cx).is_some();

            if playing {
                let position = cx.mouse_position();

                if !bounds.contains(&position) {
                    return;
                }

                let relative = cx.mouse_position() - bounds.origin;
                let relative_x = relative.x.0;
                let width = bounds.size.width.0;
                let position = (relative_x / width).clamp(0.0, 1.0);
                let seconds = position as f64 * duration as f64;

                cx.prevent_default();

                let interface = cx.global::<GPUIPlaybackInterface>();
                interface.seek(seconds);
            }
        });
    }
}
