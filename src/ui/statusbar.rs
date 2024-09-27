use gpui::*;

use crate::library::scan::ScanEvent;

use super::models::Models;

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
