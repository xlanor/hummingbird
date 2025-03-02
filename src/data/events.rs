use std::path::PathBuf;
use std::sync::Arc;

use gpui::RenderImage;

use crate::playback::queue::QueueItemUIData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageType {
    CurrentAlbumArt,
    CachedImage(u64),
    AlbumArt(i64),
    ArtistPortrait(i64),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageLayout {
    BGR,
    RGB,
}

/// A command to the data thread. This is used to control the playback thread from other threads.
/// The data thread recieves these commands from an MPSC channel, and processes them in the order
/// they are recieved, every 10 seconds.
#[derive(Debug, PartialEq, Clone)]
pub enum DataCommand {
    /// Requests that the data proccessing thread decode the specified image. The image type is
    /// used to keep track of which image is being decoded, and the layout is used to determine
    /// whether or not RGB to BGR conversion is necessary.
    DecodeImage(Box<[u8]>, ImageType, ImageLayout, bool),
    /// Requests that the data processing thread perform cache maintenance.
    EvictQueueCache,
    /// Requests that the specified file is opened and metadata is read.
    ReadMetadata(PathBuf),
}

/// An event from the data thread. This is used to communicate information from the data thread to
/// other threads. The data thread sends these events to an MPSC channel, and the main thread
/// processes them in the order they are recieved.
#[derive(Debug, Clone)]
pub enum DataEvent {
    /// Indicates that the data processing thread has decoded the specified image.
    ImageDecoded(Arc<RenderImage>, ImageType),
    /// Indicates that the data processing thread has encountered an error while decoding the
    /// specified image.
    DecodeError(ImageType),
    /// Indicates that new metadata is available for the specified file.
    MetadataRead(PathBuf, QueueItemUIData),
    /// Indicates that the data processing thread has evicted the specified images from the cache,
    /// and they should be removed from the sprite atlas.
    CacheDrops(Vec<Arc<RenderImage>>),
}
