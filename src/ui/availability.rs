use std::path::Path;

use gpui::App;

use crate::library::{db::LibraryAccess, types::Track};

pub fn is_track_path_available(path: &Path) -> bool {
    path.exists()
}

pub fn is_track_available(track: &Track) -> bool {
    is_track_path_available(&track.location)
}

pub fn has_available_tracks(tracks: &[Track]) -> bool {
    tracks.iter().any(is_track_available)
}

pub fn album_has_available_tracks(cx: &mut App, album_id: i64) -> bool {
    cx.list_tracks_in_album(album_id)
        .map(|tracks| tracks.iter().any(is_track_available))
        .unwrap_or_default()
}

pub fn artist_has_available_tracks(cx: &mut App, artist_id: i64) -> bool {
    cx.get_all_tracks_by_artist(artist_id)
        .map(|tracks| tracks.iter().any(is_track_available))
        .unwrap_or_default()
}
