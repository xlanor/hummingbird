use std::{
    io::Cursor,
    path::Path,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::sleep,
};

use ahash::{AHashMap, RandomState};
use gpui::{RenderImage, SharedString};
use image::{imageops::thumbnail, Frame};
use smallvec::SmallVec;
use tracing::{debug, warn};

use crate::{
    media::{builtin::symphonia::SymphoniaProvider, traits::MediaProvider},
    playback::queue::{DataSource, QueueItemUIData},
    util::rgb_to_bgr,
};

use super::{
    events::{DataCommand, DataEvent, ImageLayout, ImageType},
    interface::DataInterface,
};

fn create_generic_queue_item(path: &Path) -> QueueItemUIData {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|str| SharedString::from(str.to_string()));

    QueueItemUIData {
        image: None,
        name,
        artist_name: Some(SharedString::from("Unknown Artist")),
        source: DataSource::Metadata,
    }
}

pub struct DataThread {
    commands_rx: Receiver<DataCommand>,
    events_tx: Sender<DataEvent>,
    image_cache: AHashMap<u64, Arc<RenderImage>>,
    // TODO: get metadata from other providers as well
    media_provider: Box<dyn MediaProvider>,
    hash_state: RandomState,
}

impl DataThread {
    /// Starts the data thread and returns the created interface.
    pub fn start<T: DataInterface>() -> T {
        let (commands_tx, commands_rx) = std::sync::mpsc::channel();
        let (events_tx, events_rx) = std::sync::mpsc::channel();

        std::thread::Builder::new()
            .name("data".to_string())
            .spawn(move || {
                let mut thread = DataThread {
                    commands_rx,
                    events_tx,
                    image_cache: AHashMap::new(),
                    media_provider: Box::new(SymphoniaProvider::default()),
                    hash_state: RandomState::new(),
                };

                thread.run();
            })
            .expect("could not start data thread");

        T::new(commands_tx, events_rx)
    }

    fn run(&mut self) {
        while let Ok(command) = self.commands_rx.recv() {
            match command {
                DataCommand::DecodeImage(data, image_type, layout, thumb) => {
                    if self.decode_image(data, image_type, layout, thumb).is_err() {
                        self.events_tx
                            .send(DataEvent::DecodeError(image_type))
                            .expect("could not send event");
                    }
                }
                DataCommand::EvictQueueCache => self.evict_unneeded_data(),
                DataCommand::ReadMetadata(path) => {
                    let item = self.read_metadata(&path);

                    self.events_tx
                        .send(DataEvent::MetadataRead(path, item))
                        .expect("could not send event");
                }
            }

            sleep(std::time::Duration::from_millis(1));
        }
    }

    // The only real possible error here is if the image format is unsupported, or the image is
    // corrupt. In either case, there's literally nothing we can do about it, and the only
    // required information is that there was an error. So, we just return `Result<(), ()>`.
    fn decode_image(
        &self,
        data: Box<[u8]>,
        image_type: ImageType,
        image_layout: ImageLayout,
        thumb: bool,
    ) -> Result<(), ()> {
        let mut image = image::ImageReader::new(Cursor::new(data.clone()))
            .with_guessed_format()
            .map_err(|_| ())?
            .decode()
            .map_err(|_| ())?
            .into_rgba8();

        if image_layout == ImageLayout::BGR {
            rgb_to_bgr(&mut image);
        }

        self.events_tx
            .send(DataEvent::ImageDecoded(
                Arc::new(RenderImage::new(SmallVec::from_vec(vec![Frame::new(
                    if thumb {
                        thumbnail(&image, 80, 80)
                    } else {
                        image
                    },
                )]))),
                image_type,
            ))
            .expect("could not send event");

        Ok(())
    }

    fn read_metadata(&mut self, path: &PathBuf) -> QueueItemUIData {
        let file = if let Ok(file) = std::fs::File::open(path) {
            file
        } else {
            warn!("Failed to open file {:?}, queue may be desynced", path);
            warn!("Ensure the file exists before placing it in the queue");
            return create_generic_queue_item(path);
        };

        if self.media_provider.open(file, path.extension()).is_err() {
            warn!("Media provider couldn't open file, creating generic queue item");
            return create_generic_queue_item(path);
        }

        if self.media_provider.start_playback().is_err() {
            warn!("Media provider couldn't start playback, creating generic queue item");
            return create_generic_queue_item(path);
        }

        let metadata = if let Ok(metadata) = self.media_provider.read_metadata() {
            metadata.clone()
        } else {
            warn!("Media provider couldn't retrieve metadata, creating generic queue item");
            return create_generic_queue_item(path);
        };

        let album_art = self
            .media_provider
            .read_image()
            .ok()
            .flatten()
            .and_then(|v| {
                // we do this because we do not want to be storing entire encoded images
                // long-term, collisions don't particuarly matter here so the benefits outweigh
                // the tradeoffs
                let key = self.hash_state.hash_one(v.clone());

                if let Some(cached) = self.image_cache.get(&key) {
                    debug!("Image cache hit for key {}", key);
                    Some(cached.clone())
                } else {
                    debug!("Image cache miss for key {}, decoding and caching", key);
                    let mut image = image::ImageReader::new(Cursor::new(v.clone()))
                        .with_guessed_format()
                        .map_err(|_| ())
                        .ok()?
                        .decode()
                        .ok()?
                        .into_rgba8();

                    rgb_to_bgr(&mut image);

                    let value = Arc::new(RenderImage::new(SmallVec::from_vec(vec![Frame::new(
                        thumbnail(&image, 80, 80),
                    )])));
                    self.image_cache.insert(key, value.clone());

                    Some(value)
                }
            });

        // UIQueueItem {
        //     file_path: path.clone(),
        //     track_name: metadata
        //         .name
        //         .map(SharedString::from)
        //         .unwrap_or_else(|| create_generic_queue_item(path).track_name),
        //     artist_name: metadata
        //         .artist
        //         .map(SharedString::from)
        //         .unwrap_or_else(|| SharedString::from("Unknown Artist")),
        //     album_art,
        // }

        QueueItemUIData {
            image: album_art,
            name: Some(
                metadata
                    .name
                    .map(SharedString::from)
                    .unwrap_or_else(|| create_generic_queue_item(path).name.unwrap()),
            ),
            artist_name: Some(
                metadata
                    .artist
                    .map(SharedString::from)
                    .unwrap_or_else(|| SharedString::from("Unknown Artist")),
            ),
            source: DataSource::Metadata,
        }
    }

    fn evict_unneeded_data(&mut self) {
        // we have to duplicate this data in order to get around borrowing rules
        let keys: Vec<u64> = self.image_cache.keys().cloned().collect();
        let mut removed = vec![];

        for key in keys {
            let value = self.image_cache.get(&key).unwrap();

            // no clue how this could possibly be less than 2 but it doesn't hurt to check
            if Arc::<gpui::RenderImage>::strong_count(value) <= 2 {
                debug!("evicting {}", key);
                let result = self.image_cache.remove(&key);

                if let Some(result) = result {
                    removed.push(result);
                }
            }
        }

        self.events_tx
            .send(DataEvent::CacheDrops(removed))
            .expect("could not send event");
    }
}
