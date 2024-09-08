use std::sync::Arc;

use gpui::RenderImage;

use crate::media::metadata::Metadata;

#[derive(Debug, Clone)]
pub struct UIQueueItem {
    pub metadata: Metadata,
    pub file_path: String,
    pub album_art: Option<Arc<RenderImage>>,
}
