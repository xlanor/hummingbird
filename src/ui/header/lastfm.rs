use gpui::*;

use crate::ui::{
    constants::FONT_AWESOME_BRANDS,
    models::{MMBSList, Models},
    theme::Theme,
};

enum LastFMInternalState {
    Disconnected,
    AwaitingFinalization(String),
    Connected,
}

pub struct LastFM {
    mmbs: Model<MMBSList>,
    state: Model<LastFMInternalState>,
}

impl LastFM {
    pub fn new<V: 'static>(cx: &mut ViewContext<V>) -> View<Self> {
        cx.new_view(|cx| {
            let models = cx.global::<Models>();
            let mmbs = models.mmbs.clone();
            let state = cx.new_model(|cx| LastFMInternalState::Disconnected);

            LastFM { mmbs, state }
        })
    }
}

impl Render for LastFM {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .text_sm()
            .px(px(12.0))
            .pb(px(6.0))
            .pt(px(5.0))
            .text_color(theme.text_secondary)
            .bg(theme.window_button)
            .id("lastfm-button")
            .hover(|this| this.bg(theme.window_button_hover))
            .active(|this| this.bg(theme.window_button_active))
            .on_mouse_down(MouseButton::Left, |_, cx| {
                cx.prevent_default();
                cx.stop_propagation();
            })
            .child(
                div()
                    .font_family(FONT_AWESOME_BRANDS)
                    .mr(px(8.0))
                    .pt(px(3.0))
                    .text_size(px(11.0))
                    .h_full()
                    .child(""),
            )
            .child(div().child("Sign in"))
    }
}
