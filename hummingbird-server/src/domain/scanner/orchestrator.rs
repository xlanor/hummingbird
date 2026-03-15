use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use camino::Utf8PathBuf;
use tokio::sync::broadcast;
use tracing::{info, warn};

use super::decode::{extract_metadata, load_album_art, make_sortable, resolve_date};
use super::discover::discover_files;
use super::{ScanCommand, ScanHandle, ScanStatus, ScannedAlbum, ScannedTrack};
use crate::infrastructure::persistence::Database;

pub fn start_scanner(
    db: Arc<dyn Database>,
    scan_dirs: Vec<Utf8PathBuf>,
) -> ScanHandle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(8);
    let (status_tx, _) = broadcast::channel(64);
    let handle = ScanHandle {
        cmd_tx,
        status_tx: status_tx.clone(),
    };

    tokio::spawn(scanner_loop(db, scan_dirs, cmd_rx, status_tx));

    handle
}

async fn scanner_loop(
    db: Arc<dyn Database>,
    scan_dirs: Vec<Utf8PathBuf>,
    mut cmd_rx: tokio::sync::mpsc::Receiver<ScanCommand>,
    status_tx: broadcast::Sender<ScanStatus>,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            ScanCommand::Scan { force } => {
                let _ = status_tx.send(ScanStatus::Scanning {
                    processed: 0,
                    total: 0,
                });
                let count = run_scan(&db, &scan_dirs, force, &status_tx).await;
                let _ = status_tx.send(ScanStatus::Complete {
                    tracks_found: count,
                });
                info!("scan complete: {count} tracks");
            }
        }
    }
}

async fn run_scan(
    db: &Arc<dyn Database>,
    scan_dirs: &[Utf8PathBuf],
    _force: bool,
    status_tx: &broadcast::Sender<ScanStatus>,
) -> u64 {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in scan_dirs {
        discover_files(dir.as_std_path(), &mut files);
    }

    let total = files.len() as u64;
    info!("discovered {total} audio files");

    let mut processed: u64 = 0;
    let mut artist_cache: HashMap<String, i64> = HashMap::new();
    let mut album_cache: HashMap<(String, String, i64), i64> = HashMap::new();

    for path in &files {
        let location = path.to_string_lossy().to_string();

        match extract_metadata(path) {
            Some(meta) => {
                let artist_name = meta
                    .album_artist
                    .as_deref()
                    .or(meta.artist.as_deref())
                    .unwrap_or("Unknown Artist");

                let artist_id = if let Some(&id) = artist_cache.get(artist_name) {
                    id
                } else {
                    let id = match db.upsert_artist(artist_name).await {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("failed to upsert artist {artist_name}: {e}");
                            continue;
                        }
                    };
                    artist_cache.insert(artist_name.to_string(), id);
                    id
                };

                let album_title = meta.album.as_deref().unwrap_or("Unknown Album");
                let mbid = meta.mbid_album.as_deref().unwrap_or("none");
                let album_key = (album_title.to_string(), mbid.to_string(), artist_id);

                let album_id = if let Some(&id) = album_cache.get(&album_key) {
                    id
                } else {
                    let (release_date, date_precision) = resolve_date(&meta);

                    let folder = path.parent().map(|p| p.to_string_lossy().to_string());
                    let (image, thumb) = load_album_art(path);

                    let scanned = ScannedAlbum {
                        title: album_title.to_string(),
                        title_sortable: make_sortable(album_title),
                        artist_id,
                        image,
                        thumb,
                        release_date,
                        date_precision,
                        label: meta.label.clone(),
                        catalog_number: meta.catalog.clone(),
                        isrc: meta.isrc.clone(),
                        mbid: mbid.to_string(),
                        vinyl_numbering: meta.vinyl_numbering,
                    };

                    let id = match db.upsert_album(&scanned).await {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("failed to upsert album {album_title}: {e}");
                            continue;
                        }
                    };

                    if let Some(ref f) = folder {
                        let disc = meta.disc_current.unwrap_or(0) as i32;
                        let _ = db.upsert_album_path(id, f, if disc > 0 { disc } else { -1 }).await;
                    }

                    album_cache.insert(album_key, id);
                    id
                };

                let folder = path.parent().map(|p| p.to_string_lossy().to_string());

                let fallback_name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let track_title = meta.name.unwrap_or_else(|| fallback_name.clone());
                let scanned_track = ScannedTrack {
                    title_sortable: make_sortable(&track_title),
                    title: track_title,
                    album_id: Some(album_id),
                    track_number: meta.track_current.map(|n| n as i32),
                    disc_number: meta.disc_current.map(|n| n as i32),
                    duration: meta.duration,
                    location,
                    genres: meta.genre,
                    artist_names: meta.artist,
                    folder,
                };

                if let Err(e) = db.upsert_track(&scanned_track).await {
                    warn!("failed to upsert track: {e}");
                }
            }
            None => {
                warn!("failed to read metadata from {}", path.display());
            }
        }

        processed += 1;
        if processed % 50 == 0 {
            let _ = status_tx.send(ScanStatus::Scanning { processed, total });
        }
    }

    processed
}
