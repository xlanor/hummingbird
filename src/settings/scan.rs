use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSettings {
    #[serde(default = "retrieve_default_paths")]
    pub paths: Vec<PathBuf>,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            paths: retrieve_default_paths(),
        }
    }
}

fn retrieve_default_paths() -> Vec<PathBuf> {
    // TODO: we should also probably check if these directories exist
    let system_music = directories::UserDirs::new()
        .unwrap()
        .audio_dir()
        .unwrap()
        .to_path_buf();

    vec![system_music]
}
