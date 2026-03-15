use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use camino::Utf8PathBuf;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::db::Repository;
use crate::models::{ScannedAlbum, ScannedTrack};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "m4a", "aac", "wav", "aiff", "aif", "wv", "ape",
];

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

pub fn start_scanner(
    repo: Arc<dyn Repository>,
    scan_dirs: Vec<Utf8PathBuf>,
) -> ScanHandle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(8);
    let (status_tx, _) = broadcast::channel(64);
    let handle = ScanHandle {
        cmd_tx,
        status_tx: status_tx.clone(),
    };

    tokio::spawn(scanner_loop(repo, scan_dirs, cmd_rx, status_tx));

    handle
}

async fn scanner_loop(
    repo: Arc<dyn Repository>,
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
                let count = run_scan(&repo, &scan_dirs, force, &status_tx).await;
                let _ = status_tx.send(ScanStatus::Complete {
                    tracks_found: count,
                });
                info!("scan complete: {count} tracks");
            }
        }
    }
}

async fn run_scan(
    repo: &Arc<dyn Repository>,
    scan_dirs: &[Utf8PathBuf],
    _force: bool,
    status_tx: &broadcast::Sender<ScanStatus>,
) -> u64 {
    // Discover files
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
                // Resolve artist
                let artist_name = meta
                    .album_artist
                    .as_deref()
                    .or(meta.artist.as_deref())
                    .unwrap_or("Unknown Artist");

                let artist_id = if let Some(&id) = artist_cache.get(artist_name) {
                    id
                } else {
                    let id = match repo.upsert_artist(artist_name).await {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("failed to upsert artist {artist_name}: {e}");
                            continue;
                        }
                    };
                    artist_cache.insert(artist_name.to_string(), id);
                    id
                };

                // Resolve album
                let album_title = meta.album.as_deref().unwrap_or("Unknown Album");
                let mbid = meta.mbid_album.as_deref().unwrap_or("none");
                let album_key = (album_title.to_string(), mbid.to_string(), artist_id);

                let album_id = if let Some(&id) = album_cache.get(&album_key) {
                    id
                } else {
                    let (release_date, date_precision) = resolve_date(&meta);

                    // Try to find album art from the file's directory
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

                    let id = match repo.upsert_album(&scanned).await {
                        Ok(id) => id,
                        Err(e) => {
                            warn!("failed to upsert album {album_title}: {e}");
                            continue;
                        }
                    };

                    // Create album_path entry
                    if let Some(ref f) = folder {
                        let disc = meta.disc_current.unwrap_or(0) as i32;
                        let _ = repo.upsert_album_path(id, f, if disc > 0 { disc } else { -1 }).await;
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

                if let Err(e) = repo.upsert_track(&scanned_track).await {
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

fn discover_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_files(&path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                out.push(path);
            }
        }
    }
}

struct FileMetadata {
    name: Option<String>,
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    track_current: Option<u64>,
    disc_current: Option<u64>,
    duration: i64,
    date: Option<chrono::DateTime<chrono::Utc>>,
    year: Option<u16>,
    vinyl_numbering: bool,
    label: Option<String>,
    catalog: Option<String>,
    isrc: Option<String>,
    mbid_album: Option<String>,
}

fn extract_metadata(path: &Path) -> Option<FileMetadata> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let mut meta = FileMetadata {
        name: None,
        artist: None,
        album_artist: None,
        album: None,
        genre: None,
        track_current: None,
        disc_current: None,
        duration: 0,
        date: None,
        year: None,
        vinyl_numbering: false,
        label: None,
        catalog: None,
        isrc: None,
        mbid_album: None,
    };

    // Extract duration from the default track
    if let Some(track) = probed.format.default_track() {
        if let Some(n_frames) = track.codec_params.n_frames {
            if let Some(rate) = track.codec_params.sample_rate {
                meta.duration = (n_frames as f64 / rate as f64 * 1000.0) as i64;
            }
        }
        if let Some(tb) = track.codec_params.time_base {
            if let Some(n_frames) = track.codec_params.n_frames {
                let time = tb.calc_time(n_frames);
                meta.duration = (time.seconds as i64 * 1000) + (time.frac * 1000.0) as i64;
            }
        }
    }

    // Extract tags from metadata
    let collect_tags = |revision: &symphonia::core::meta::MetadataRevision, meta: &mut FileMetadata| {
        for tag in revision.tags() {
            let key = tag.std_key;
            let val = tag.value.to_string();
            if val.is_empty() {
                continue;
            }
            match key {
                Some(symphonia::core::meta::StandardTagKey::TrackTitle) => meta.name = Some(val),
                Some(symphonia::core::meta::StandardTagKey::Artist) => meta.artist = Some(val),
                Some(symphonia::core::meta::StandardTagKey::AlbumArtist) => {
                    meta.album_artist = Some(val)
                }
                Some(symphonia::core::meta::StandardTagKey::Album) => meta.album = Some(val),
                Some(symphonia::core::meta::StandardTagKey::Genre) => meta.genre = Some(val),
                Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                    meta.track_current = val.split('/').next().and_then(|s| s.parse().ok());
                }
                Some(symphonia::core::meta::StandardTagKey::DiscNumber) => {
                    meta.disc_current = val.split('/').next().and_then(|s| s.parse().ok());
                }
                Some(symphonia::core::meta::StandardTagKey::Date) => {
                    meta.date = chrono::DateTime::parse_from_rfc3339(&val)
                        .ok()
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .or_else(|| {
                            // Try parsing just a date
                            chrono::NaiveDate::parse_from_str(&val, "%Y-%m-%d")
                                .ok()
                                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                        });
                    // Also try to get year from date string
                    if meta.year.is_none() {
                        meta.year = val.get(..4).and_then(|y| y.parse().ok());
                    }
                }
                Some(symphonia::core::meta::StandardTagKey::Label) => meta.label = Some(val),
                Some(symphonia::core::meta::StandardTagKey::IdentCatalogNumber) => {
                    meta.catalog = Some(val)
                }
                Some(symphonia::core::meta::StandardTagKey::IdentIsrc) => meta.isrc = Some(val),
                Some(symphonia::core::meta::StandardTagKey::MusicBrainzAlbumId) => {
                    meta.mbid_album = Some(val)
                }
                _ => {
                    // Check by key name for tags not in StandardTagKey
                    let key_str = tag.key.to_lowercase();
                    if key_str == "musicbrainz album id" || key_str == "musicbrainz_albumid" {
                        meta.mbid_album = Some(val);
                    }
                }
            }
        }
    };

    // Check metadata from the format reader
    if let Some(md) = probed.format.metadata().current() {
        collect_tags(md, &mut meta);
    }

    // Also check the probe's metadata
    if let Some(md) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        collect_tags(md, &mut meta);
    }

    Some(meta)
}

fn load_album_art(track_path: &Path) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let dir = match track_path.parent() {
        Some(d) => d,
        None => return (None, None),
    };

    // Look for cover art files in the directory
    let art_names = ["cover", "front", "folder", "album"];
    let art_exts = ["jpg", "jpeg", "png", "bmp"];

    for name in &art_names {
        for ext in &art_exts {
            let art_path = dir.join(format!("{name}.{ext}"));
            if art_path.exists() {
                if let Ok(data) = std::fs::read(&art_path) {
                    let thumb = make_thumbnail(&data);
                    return (Some(data), thumb);
                }
            }
        }
    }

    // Try to extract embedded art from the audio file
    if let Some(art) = extract_embedded_art(track_path) {
        let thumb = make_thumbnail(&art);
        return (Some(art), thumb);
    }

    (None, None)
}

fn extract_embedded_art(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let check_visuals = |revision: &symphonia::core::meta::MetadataRevision| -> Option<Vec<u8>> {
        revision.visuals().first().map(|v| v.data.to_vec())
    };

    if let Some(md) = probed.format.metadata().current() {
        if let Some(art) = check_visuals(md) {
            return Some(art);
        }
    }
    if let Some(md) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        if let Some(art) = check_visuals(md) {
            return Some(art);
        }
    }
    None
}

fn make_thumbnail(data: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(data).ok()?;
    let thumb = img.thumbnail(70, 70);
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    thumb
        .write_to(&mut cursor, image::ImageFormat::Bmp)
        .ok()?;
    Some(buf)
}

fn resolve_date(meta: &FileMetadata) -> (Option<String>, Option<i32>) {
    if let Some(ref dt) = meta.date {
        let date_str = dt.format("%Y-%m-%d").to_string();
        (Some(date_str), Some(1)) // precision 1 = full date
    } else if let Some(year) = meta.year {
        let date_str = format!("{year:04}-01-01");
        (Some(date_str), Some(0)) // precision 0 = year only
    } else {
        (None, None)
    }
}

fn make_sortable(name: &str) -> String {
    let lower = name.to_lowercase();
    if let Some(rest) = lower.strip_prefix("the ") {
        rest.to_string()
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_make_sortable_strips_the() {
        assert_eq!(make_sortable("The Beatles"), "beatles");
    }

    #[test]
    fn test_make_sortable_lowercases() {
        assert_eq!(make_sortable("Pink Floyd"), "pink floyd");
    }

    #[test]
    fn test_make_sortable_no_prefix() {
        assert_eq!(make_sortable("Radiohead"), "radiohead");
    }

    #[test]
    fn test_make_sortable_the_only() {
        assert_eq!(make_sortable("The"), "the");
    }

    #[test]
    fn test_resolve_date_full_date() {
        let meta = FileMetadata {
            name: None, artist: None, album_artist: None, album: None,
            genre: None, track_current: None, disc_current: None, duration: 0,
            date: Some(chrono::DateTime::parse_from_rfc3339("2023-06-15T00:00:00Z")
                .unwrap().with_timezone(&chrono::Utc)),
            year: None, vinyl_numbering: false, label: None, catalog: None,
            isrc: None, mbid_album: None,
        };
        let (date, precision) = resolve_date(&meta);
        assert_eq!(date, Some("2023-06-15".to_string()));
        assert_eq!(precision, Some(1));
    }

    #[test]
    fn test_resolve_date_year_only() {
        let meta = FileMetadata {
            name: None, artist: None, album_artist: None, album: None,
            genre: None, track_current: None, disc_current: None, duration: 0,
            date: None, year: Some(1999), vinyl_numbering: false, label: None,
            catalog: None, isrc: None, mbid_album: None,
        };
        let (date, precision) = resolve_date(&meta);
        assert_eq!(date, Some("1999-01-01".to_string()));
        assert_eq!(precision, Some(0));
    }

    #[test]
    fn test_resolve_date_none() {
        let meta = FileMetadata {
            name: None, artist: None, album_artist: None, album: None,
            genre: None, track_current: None, disc_current: None, duration: 0,
            date: None, year: None, vinyl_numbering: false, label: None,
            catalog: None, isrc: None, mbid_album: None,
        };
        let (date, precision) = resolve_date(&meta);
        assert_eq!(date, None);
        assert_eq!(precision, None);
    }

    #[test]
    fn test_discover_files_finds_audio() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("song.flac"), b"fake").unwrap();
        fs::write(dir.path().join("song.mp3"), b"fake").unwrap();
        fs::write(dir.path().join("readme.txt"), b"fake").unwrap();
        fs::write(dir.path().join("image.png"), b"fake").unwrap();

        let mut files = Vec::new();
        discover_files(dir.path(), &mut files);
        assert_eq!(files.len(), 2);
        let exts: Vec<_> = files.iter()
            .filter_map(|p| p.extension().and_then(|e| e.to_str()))
            .collect();
        assert!(exts.contains(&"flac"));
        assert!(exts.contains(&"mp3"));
    }

    #[test]
    fn test_discover_files_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("artist").join("album");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("track.wav"), b"fake").unwrap();

        let mut files = Vec::new();
        discover_files(dir.path(), &mut files);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("track.wav"));
    }

    #[test]
    fn test_discover_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = Vec::new();
        discover_files(dir.path(), &mut files);
        assert!(files.is_empty());
    }

    #[test]
    fn test_discover_files_nonexistent_dir() {
        let mut files = Vec::new();
        discover_files(Path::new("/nonexistent/path/12345"), &mut files);
        assert!(files.is_empty());
    }

    #[test]
    fn test_discover_files_case_insensitive_extension() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("song.FLAC"), b"fake").unwrap();
        fs::write(dir.path().join("song.Mp3"), b"fake").unwrap();

        let mut files = Vec::new();
        discover_files(dir.path(), &mut files);
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_supported_extensions_coverage() {
        assert!(SUPPORTED_EXTENSIONS.contains(&"flac"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"mp3"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"ogg"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"opus"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"m4a"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"wav"));
        assert!(SUPPORTED_EXTENSIONS.contains(&"aiff"));
        assert!(!SUPPORTED_EXTENSIONS.contains(&"txt"));
        assert!(!SUPPORTED_EXTENSIONS.contains(&"png"));
    }
}
