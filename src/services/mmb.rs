pub mod lastfm;

use crate::media::metadata::Metadata;
use async_trait::async_trait;

/// MediaMetadataBroadcastService is a trait that can be implemented by services that wish to
/// display information about the currently playing track. When the currently playing track
/// changes, the service will be provided with the track's metadata, duration, and current
/// playback position.
///
/// The service is responsible for displaying this information in the appropriate manner. For
/// example, a service providing desktop integration should update immediately, while a service
/// that provides scrobbling functionality might want to wait some time before recording the
/// scrobble.
///
/// Note that MMBS operations are performed on the UI thread, and thus services should not perform
/// substantial blocking operations in their MMBS implementations. If, for example, a network
/// request is needed, use an async function to perform the request.
#[async_trait]
pub trait MediaMetadataBroadcastService {
    /// Called when a new track is played.
    async fn new_track(&self, info: Metadata, file_path: String);
    /// Called when the currently playing track is paused.
    async fn track_paused(&self);
    /// Called when the currently playing track is resumed.
    async fn track_resumed(&self);
    /// Called when the currently playing track is stopped.
    async fn track_stopped(&self);
    /// Called when the position of the currently playing track changes, or when a new track is
    /// played. Timestamp is in seconds.
    async fn position_changed(&self, position: u64, duration: u64);
}
