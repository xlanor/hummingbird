use std::path::Path;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub(crate) struct FileMetadata {
    pub name: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub track_current: Option<u64>,
    pub disc_current: Option<u64>,
    pub duration: i64,
    pub date: Option<chrono::DateTime<chrono::Utc>>,
    pub year: Option<u16>,
    pub vinyl_numbering: bool,
    pub label: Option<String>,
    pub catalog: Option<String>,
    pub isrc: Option<String>,
    pub mbid_album: Option<String>,
}

pub(crate) fn extract_metadata(path: &Path) -> Option<FileMetadata> {
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
                            chrono::NaiveDate::parse_from_str(&val, "%Y-%m-%d")
                                .ok()
                                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                        });
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
                    let key_str = tag.key.to_lowercase();
                    if key_str == "musicbrainz album id" || key_str == "musicbrainz_albumid" {
                        meta.mbid_album = Some(val);
                    }
                }
            }
        }
    };

    if let Some(md) = probed.format.metadata().current() {
        collect_tags(md, &mut meta);
    }

    if let Some(md) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        collect_tags(md, &mut meta);
    }

    Some(meta)
}

pub(crate) fn load_album_art(track_path: &Path) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let dir = match track_path.parent() {
        Some(d) => d,
        None => return (None, None),
    };

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

pub(crate) fn resolve_date(meta: &FileMetadata) -> (Option<String>, Option<i32>) {
    if let Some(ref dt) = meta.date {
        let date_str = dt.format("%Y-%m-%d").to_string();
        (Some(date_str), Some(1))
    } else if let Some(year) = meta.year {
        let date_str = format!("{year:04}-01-01");
        (Some(date_str), Some(0))
    } else {
        (None, None)
    }
}

pub(crate) fn make_sortable(name: &str) -> String {
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
}
