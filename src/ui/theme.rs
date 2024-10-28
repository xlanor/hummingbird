use gpui::{rgb, rgba, Global, Rgba};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(default)]
pub struct Theme {
    pub background_primary: Rgba,
    pub background_secondary: Rgba,
    pub background_tertiary: Rgba,

    pub border_color: Rgba,

    pub album_art_background: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,

    pub nav_button_hover: Rgba,
    pub nav_button_active: Rgba,

    pub playback_button: Rgba,
    pub playback_button_hover: Rgba,
    pub playback_button_active: Rgba,
    pub playback_button_border: Rgba,

    pub window_button: Rgba,
    pub window_button_hover: Rgba,
    pub window_button_active: Rgba,

    pub close_button: Rgba,
    pub close_button_hover: Rgba,
    pub close_button_active: Rgba,

    pub queue_item: Rgba,
    pub queue_item_hover: Rgba,
    pub queue_item_active: Rgba,
    pub queue_item_current: Rgba,

    pub button_primary: Rgba,
    pub button_primary_hover: Rgba,
    pub button_primary_active: Rgba,
    pub button_primary_text: Rgba,

    pub button_secondary: Rgba,
    pub button_secondary_hover: Rgba,
    pub button_secondary_active: Rgba,
    pub button_secondary_text: Rgba,

    pub button_warning: Rgba,
    pub button_warning_hover: Rgba,
    pub button_warning_active: Rgba,
    pub button_warning_text: Rgba,

    pub button_danger: Rgba,
    pub button_danger_hover: Rgba,
    pub button_danger_active: Rgba,
    pub button_danger_text: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        // TODO: Theme for scrubber (when scrubber is rewritten)
        Self {
            background_primary: rgb(0x030712),
            background_secondary: rgb(0x111827),
            background_tertiary: rgb(0x1e293b),

            border_color: rgb(0x1e293b),

            album_art_background: rgb(0x4b5563),
            text: rgb(0xf1f5f9),
            text_secondary: rgb(0xd1d5db),

            nav_button_hover: rgb(0x1e293b),
            nav_button_active: rgb(0x111827),

            playback_button: rgb(0x1f2937),
            playback_button_hover: rgb(0x374151),
            playback_button_active: rgb(0x111827),
            playback_button_border: rgb(0x374151),

            window_button: rgba(0x33415500),
            window_button_hover: rgb(0x334155),
            window_button_active: rgb(0x111827),

            queue_item: rgb(0x1e293b00),
            queue_item_hover: rgb(0x1f2937),
            queue_item_active: rgb(0x030712),
            queue_item_current: rgb(0x1f2937),

            close_button: rgba(0x33415500),
            close_button_hover: rgb(0x991b1b),
            close_button_active: rgb(0x111827),

            button_primary: rgb(0x1e3a8a),
            button_primary_hover: rgb(0x1e40af),
            button_primary_active: rgb(0x172554),
            button_primary_text: rgb(0xeff6ff),

            button_secondary: rgb(0x1f2937),
            button_secondary_hover: rgb(0x334155),
            button_secondary_active: rgb(0x0f172a),
            button_secondary_text: rgb(0xf1f5f9),

            button_warning: rgb(0x854d0e),
            button_warning_hover: rgb(0xa16207),
            button_warning_active: rgb(0x713f12),
            button_warning_text: rgb(0xfefce8),

            button_danger: rgb(0x7f1d1d),
            button_danger_hover: rgb(0x991b1b),
            button_danger_active: rgb(0x450a0a),
            button_danger_text: rgb(0xfef2f2),
        }
    }
}

impl Global for Theme {}
