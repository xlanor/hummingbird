use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use gpui::{App, AppContext, Entity};

use crate::ui::{
    app::DropImageDummyModel,
    models::{ImageTransfer, Models},
    util::drop_image_from_app,
};

use super::events::{DataCommand, DataEvent, ImageLayout, ImageType};

/// The DataInterface trait defines the method used to create the struct that will be used to
/// communicate between the data thread and the main thread.
pub trait DataInterface {
    fn new(commands_tx: Sender<DataCommand>, events_rx: Receiver<DataEvent>) -> Self;
}

pub struct GPUIDataInterface {
    commands_tx: Sender<DataCommand>,
    events_rx: Option<Receiver<DataEvent>>,
}

impl gpui::Global for GPUIDataInterface {}

/// The data interface struct that will be used to communicate between the data thread and the main
/// thread. This implementation takes advantage of the GPUI Global trait to allow any function (so
/// long as it is running on the main thread) to send commands to the data thread.
///
/// This interface takes advantage of GPUI's asynchronous runtime to read messages without blocking
/// rendering. Messages are read at quickest every 10ms, however the runtime may choose to run the
/// function that reads events less frequently, depending on the current workload. Because of this,
/// event handling should not perform any heavy operations, which should be added to the data
/// thread.
impl DataInterface for GPUIDataInterface {
    fn new(commands_tx: Sender<DataCommand>, events_rx: Receiver<DataEvent>) -> Self {
        Self {
            commands_tx,
            events_rx: Some(events_rx),
        }
    }
}

impl GPUIDataInterface {
    pub fn decode_image(
        &self,
        data: Box<[u8]>,
        image_type: ImageType,
        image_layout: ImageLayout,
        thumb: bool,
    ) {
        self.commands_tx
            .send(DataCommand::DecodeImage(
                data,
                image_type,
                image_layout,
                thumb,
            ))
            .expect("could not send tx");
    }

    pub fn evict_cache(&self) {
        self.commands_tx
            .send(DataCommand::EvictQueueCache)
            .expect("could not send tx");
    }

    pub fn get_metadata(&self, path: String) {
        self.commands_tx
            .send(DataCommand::ReadMetadata(path))
            .expect("could not send tx");
    }

    /// Starts the broadcast loop that will read events from the data thread and update data models
    /// accordingly. This function should be called once, and will panic if called more than once.
    pub fn start_broadcast(&mut self, cx: &mut App, drop_model: Entity<DropImageDummyModel>) {
        let mut events_rx = None;
        std::mem::swap(&mut self.events_rx, &mut events_rx);

        let albumart_model = cx.global::<Models>().albumart.clone();
        let queue_model = cx.global::<Models>().queue.clone();
        let image_transfer_model = cx.global::<Models>().image_transfer_model.clone();

        let Some(events_rx) = events_rx else {
            panic!("broadcast thread already started");
        };

        cx.spawn(|mut cx| async move {
            loop {
                while let Ok(event) = events_rx.try_recv() {
                    match event {
                        DataEvent::ImageDecoded(v, image_type) => match image_type {
                            ImageType::CurrentAlbumArt => {
                                albumart_model
                                    .update(&mut cx, |m, cx| {
                                        *m = Some(v);
                                        cx.notify()
                                    })
                                    .expect("failed to update albumart");
                            }
                            _ => image_transfer_model
                                .update(&mut cx, |_, cx| cx.emit(ImageTransfer(image_type, v)))
                                .expect("failed to transfer image"),
                        },
                        DataEvent::DecodeError(image_type) => match image_type {
                            ImageType::CurrentAlbumArt => {
                                albumart_model
                                    .update(&mut cx, |m, cx| {
                                        *m = None;
                                        cx.notify()
                                    })
                                    .expect("failed to update albumart");
                            }
                            _ => todo!(),
                        },
                        DataEvent::MetadataRead(path, item) => {
                            queue_model
                                .update(&mut cx, |_, cx| {
                                    cx.emit((path, item));
                                })
                                .expect("failed to update queue");
                        }
                        DataEvent::CacheDrops(vec) => drop_model
                            .update(&mut cx, |_, cx| cx.emit(vec))
                            .expect("failed to promote to cx"),
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
        })
        .detach();
    }
}
