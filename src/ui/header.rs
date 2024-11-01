use gpui::*;
use prelude::FluentBuilder;

use crate::library::scan::ScanEvent;

use super::{constants::APP_ROUNDING, models::Models, theme::Theme};

pub struct Header {
    scan_model: Model<ScanEvent>,
}

impl Header {
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

impl Render for Header {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let decorations = cx.window_decorations();
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .w_full()
            .min_h(px(32.0))
            .max_h(px(32.0))
            .bg(theme.background_secondary)
            .pl(px(12.0))
            .pb(px(6.0))
            .pt(px(4.0))
            .text_sm()
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
