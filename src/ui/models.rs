use std::sync::Arc;

use ahash::AHashMap;
use async_std::sync::Mutex;
use gpui::{AppContext, Context, EventEmitter, Global, Model, RenderImage};
use tracing::debug;

use crate::{
    data::{
        events::{ImageLayout, ImageType},
        interface::GPUIDataInterface,
        types::UIQueueItem,
    },
    library::scan::ScanEvent,
    media::metadata::Metadata,
    playback::thread::PlaybackState,
    services::mmb::MediaMetadataBroadcastService,
};

// yes this looks a little silly
impl EventEmitter<Metadata> for Metadata {}

#[derive(Debug, PartialEq, Clone)]
pub struct ImageEvent(pub Box<[u8]>);

impl EventEmitter<ImageEvent> for Option<Arc<RenderImage>> {}

pub struct Models {
    pub metadata: Model<Metadata>,
    pub albumart: Model<Option<Arc<RenderImage>>>,
    pub queue: Model<Queue>,
    pub image_transfer_model: Model<TransferDummy>,
    pub scan_state: Model<ScanEvent>,
    pub mmbs: Model<MMBSList>,
}

impl Global for Models {}

#[derive(Clone)]
pub struct PlaybackInfo {
    pub position: Model<u64>,
    pub duration: Model<u64>,
    pub playback_state: Model<PlaybackState>,
    pub current_track: Model<Option<String>>,
    pub shuffling: Model<bool>,
    pub volume: Model<f64>,
}

impl Global for PlaybackInfo {}

pub struct ImageTransfer(pub ImageType, pub Arc<RenderImage>);
pub struct TransferDummy;

impl EventEmitter<ImageTransfer> for TransferDummy {}

#[derive(Debug, PartialEq, Clone)]
pub struct Queue(pub Vec<String>);

impl EventEmitter<UIQueueItem> for Queue {}

#[derive(Clone)]
pub struct MMBSList(pub AHashMap<String, Arc<Mutex<dyn MediaMetadataBroadcastService>>>);

#[derive(Clone)]
pub enum MMBSEvent {
    NewTrack(Arc<Metadata>, String),
    TrackPaused,
    TrackResumed,
    TrackStopped,
    PositionChanged { position: u64, duration: u64 },
}

impl EventEmitter<MMBSEvent> for MMBSList {}

pub fn build_models(cx: &mut AppContext) {
    debug!("Building models");
    let metadata: Model<Metadata> = cx.new_model(|_| Metadata::default());
    let albumart: Model<Option<Arc<RenderImage>>> = cx.new_model(|_| None);
    let queue: Model<Queue> = cx.new_model(|_| Queue(Vec::new()));
    let image_transfer_model: Model<TransferDummy> = cx.new_model(|_| TransferDummy);
    let scan_state: Model<ScanEvent> = cx.new_model(|_| ScanEvent::ScanCompleteIdle);
    let mmbs: Model<MMBSList> = cx.new_model(|_| MMBSList(AHashMap::new()));

    cx.subscribe(&albumart, |_, ev, cx| {
        let img = ev.0.clone();
        cx.global::<GPUIDataInterface>().decode_image(
            img,
            ImageType::CurrentAlbumArt,
            ImageLayout::BGR,
            true,
        );
    })
    .detach();

    cx.subscribe(&mmbs, |m, ev, cx| {
        let list = m.read(cx);

        // cloning actually is neccesary because of the async move closure
        #[allow(clippy::unnecessary_to_owned)]
        for mmbs in list.0.values().cloned() {
            let ev = ev.clone();
            cx.spawn(|_| async move {
                let borrow = mmbs.lock().await;
                match ev {
                    MMBSEvent::NewTrack(metadata, path) => borrow.new_track(metadata, path),
                    MMBSEvent::TrackPaused => borrow.track_paused(),
                    MMBSEvent::TrackResumed => borrow.track_resumed(),
                    MMBSEvent::TrackStopped => borrow.track_stopped(),
                    MMBSEvent::PositionChanged { position, duration } => {
                        borrow.position_changed(position, duration)
                    }
                }
                .await;
            })
            .detach();
        }
    })
    .detach();

    cx.set_global(Models {
        metadata,
        albumart,
        queue,
        image_transfer_model,
        scan_state,
        mmbs,
    });

    let position: Model<u64> = cx.new_model(|_| 0);
    let duration: Model<u64> = cx.new_model(|_| 0);
    let playback_state: Model<PlaybackState> = cx.new_model(|_| PlaybackState::Stopped);
    let current_track: Model<Option<String>> = cx.new_model(|_| None);
    let shuffling: Model<bool> = cx.new_model(|_| false);
    let volume: Model<f64> = cx.new_model(|_| 1.0);

    cx.set_global(PlaybackInfo {
        position,
        duration,
        playback_state,
        current_track,
        shuffling,
        volume,
    });
}
