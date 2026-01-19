use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px,
};

use crate::{
    settings::{Settings, SettingsGlobal, save_settings},
    ui::components::{checkbox::checkbox, label::label, section_header::section_header},
};

pub struct PlaybackSettings {
    settings: Entity<Settings>,
}

impl PlaybackSettings {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let settings = cx.global::<SettingsGlobal>().model.clone();
            cx.observe(&settings, |_, _, cx| cx.notify()).detach();

            Self { settings }
        })
    }

    fn update_playback(
        &self,
        cx: &mut App,
        update: impl FnOnce(&mut crate::settings::playback::PlaybackSettings),
    ) {
        self.settings.update(cx, move |settings, cx| {
            update(&mut settings.playback);

            let settings_path = cx.global::<SettingsGlobal>().path.clone();
            save_settings(&settings_path, settings);
            cx.notify();
        });
    }
}

impl Render for PlaybackSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playback = self.settings.read(cx).playback.clone();

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(section_header("Playback"))
            .child(
                label("playback-always-repeat", "Always repeat")
                    .subtext("Disables the \"Off\" repeat mode.")
                    .cursor_pointer()
                    .w_full()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_playback(cx, |playback| {
                            playback.always_repeat = !playback.always_repeat;
                        });
                    }))
                    .child(checkbox(
                        "playback-always-repeat-check",
                        playback.always_repeat,
                    )),
            )
            .child(
                label(
                    "playback-prev-track-jump-first",
                    "Previous button jumps to the beginning of the track if less than 5 seconds \
                    has elapsed",
                )
                .cursor_pointer()
                .w_full()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.update_playback(cx, |playback| {
                        playback.prev_track_jump_first = !playback.prev_track_jump_first;
                    });
                }))
                .child(checkbox(
                    "playback-prev-track-jump-first-check",
                    playback.prev_track_jump_first,
                )),
            )
    }
}
