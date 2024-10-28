use gpui::{px, rgb};

#[cfg(target_os = "windows")]
pub const FONT_AWESOME: &str = "Font Awesome 6 Free Solid";
#[cfg(not(target_os = "windows"))]
pub const FONT_AWESOME: &str = "Font Awesome 6 Free";

pub const APP_ROUNDING: gpui::Pixels = px(6.0);
