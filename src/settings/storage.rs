use gpui::{Pixels, px};
use serde::{Deserialize, Serialize};

use crate::ui::models::CurrentTrack;

use std::{fs, path::PathBuf};

pub const DEFAULT_SIDEBAR_WIDTH: Pixels = px(225.0);
pub const DEFAULT_QUEUE_WIDTH: Pixels = px(275.0);

fn default_sidebar_width() -> f32 {
    f32::from(DEFAULT_SIDEBAR_WIDTH)
}

fn default_queue_width() -> f32 {
    f32::from(DEFAULT_QUEUE_WIDTH)
}

/// Data to store while quitting the app
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageData {
    pub current_track: Option<CurrentTrack>,
    /// Width of the library sidebar in pixels
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    /// Width of the queue panel in pixels
    #[serde(default = "default_queue_width")]
    pub queue_width: f32,
}

impl StorageData {
    pub fn sidebar_width(&self) -> Pixels {
        px(self.sidebar_width)
    }

    pub fn queue_width(&self) -> Pixels {
        px(self.queue_width)
    }
}

impl Default for StorageData {
    fn default() -> Self {
        Self {
            current_track: None,
            sidebar_width: f32::from(DEFAULT_SIDEBAR_WIDTH),
            queue_width: f32::from(DEFAULT_QUEUE_WIDTH),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Storage {
    /// File path to store data
    path: PathBuf,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Save `StorageData` on file system
    pub fn save(&self, data: &StorageData) {
        // save into file
        let result = fs::File::create(self.path.clone())
            .and_then(|file| serde_json::to_writer(file, &data).map_err(|e| e.into()));
        // ignore error, but log it
        if let Err(e) = result {
            tracing::warn!("could not save `AppState` {:?}", e);
        };
    }

    /// Load `StorageData` from storage or use `StorageData::default` in case of any errors
    pub fn load_or_default(&self) -> StorageData {
        std::fs::File::open(self.path.clone())
            .and_then(|file| {
                serde_json::from_reader(file)
                    .map_err(|e| e.into())
                    .map(|data: StorageData| match &data.current_track {
                        // validate whether path still exists
                        Some(current_track) if !current_track.get_path().exists() => StorageData {
                            current_track: None,
                            // Preserve other settings when invalidating current_track
                            sidebar_width: data.sidebar_width,
                            queue_width: data.queue_width,
                        },
                        _ => data,
                    })
            })
            .unwrap_or_default()
    }
}
