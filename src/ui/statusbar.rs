use gpui::*;
use prelude::FluentBuilder;

use crate::library::scan::ScanEvent;

use super::{constants::APP_ROUNDING, models::Models};

pub struct StatusBar {
    scan_model: Model<ScanEvent>,
}

impl StatusBar {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        let scan_model = cx.global::<Models>().scan_state.clone();

        cx.new_view(|cx| {
            cx.observe(&scan_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { scan_model }
        })
    }
}

impl Render for StatusBar {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let decorations = cx.window_decorations();

        div()
            .flex()
            .w_full()
            .bg(rgb(0x111827))
            .pl(px(12.0))
            .pb(px(6.0))
            .pt(px(4.0))
            .text_sm()
            .border_t_1()
            .border_color(rgb(0x1e293b))
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling } => div
                    .when(!(tiling.bottom || tiling.left), |div| {
                        div.rounded_bl(APP_ROUNDING)
                    })
                    .when(!(tiling.bottom || tiling.right), |div| {
                        div.rounded_br(APP_ROUNDING)
                    }),
            })
            .child(match self.scan_model.read(cx) {
                ScanEvent::ScanCompleteIdle => "Not scanning".to_string(),
                ScanEvent::ScanProgress { current, total } => {
                    format!(
                        "Scanning... ({}%)",
                        (*current as f64 / *total as f64 * 100.0).round()
                    )
                }
                ScanEvent::DiscoverProgress(progress) => {
                    format!("Discovered {} files", progress)
                }
                ScanEvent::Cleaning => "Checking for changes".to_string(),
                ScanEvent::ScanCompleteWatching => "Watching for new files".to_string(),
            })
    }
}
