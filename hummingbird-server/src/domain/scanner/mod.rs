pub mod dao;
pub mod decode;
pub mod discover;
pub mod orchestrator;

use tokio::sync::broadcast;

use super::library::Track;

pub struct ScannedTrack {
    pub title: String,
    pub title_sortable: String,
    pub album_id: Option<i64>,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub duration: i64,
    pub location: String,
    pub genres: Option<String>,
    pub artist_names: Option<String>,
    pub folder: Option<String>,
}

pub struct ScannedAlbum {
    pub title: String,
    pub title_sortable: String,
    pub artist_id: i64,
    pub image: Option<Vec<u8>>,
    pub thumb: Option<Vec<u8>>,
    pub release_date: Option<String>,
    pub date_precision: Option<i32>,
    pub label: Option<String>,
    pub catalog_number: Option<String>,
    pub isrc: Option<String>,
    pub mbid: String,
    pub vinyl_numbering: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum ScanStatus {
    Idle,
    Scanning { processed: u64, total: u64 },
    Complete { tracks_found: u64 },
}

pub struct ScanHandle {
    cmd_tx: tokio::sync::mpsc::Sender<ScanCommand>,
    status_tx: broadcast::Sender<ScanStatus>,
}

enum ScanCommand {
    Scan { force: bool },
}

impl ScanHandle {
    pub fn trigger_scan(&self, force: bool) {
        let _ = self.cmd_tx.try_send(ScanCommand::Scan { force });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ScanStatus> {
        self.status_tx.subscribe()
    }
}
