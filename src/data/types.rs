use std::sync::Arc;

use gpui::{RenderImage, SharedString};

#[derive(Debug, Clone)]
pub struct UIQueueItem {
    pub track_name: SharedString,
    pub artist_name: SharedString,
    pub file_path: String,
    pub album_art: Option<Arc<RenderImage>>,
}
