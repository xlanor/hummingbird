use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "opus", "m4a", "aac", "wav", "aiff", "aif", "wv", "ape",
];

pub fn discover_files(dir: &Path, out: &mut Vec<PathBuf>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
