use std::{
    sync::mpsc::{Receiver, Sender},
    thread::sleep,
};

use rand::{seq::SliceRandom, thread_rng};
use tracing::{debug, info};

#[cfg(target_os = "linux")]
use crate::devices::builtin::pulse::PulseProvider;

#[cfg(not(target_os = "linux"))]
use crate::device::builtin::cpal::CpalProvider;

use crate::{
    devices::{
        format::{ChannelSpec, FormatInfo},
        resample::Resampler,
        traits::{Device, DeviceProvider, OutputStream},
    },
    media::{
        builtin::symphonia::SymphoniaProvider, errors::PlaybackReadError, traits::MediaProvider,
    },
};

use super::{
    events::{PlaybackCommand, PlaybackEvent},
    interface::PlaybackInterface,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub struct PlaybackThread {
    commands_rx: Receiver<PlaybackCommand>,
    events_tx: Sender<PlaybackEvent>,
    media_provider: Option<Box<dyn MediaProvider>>,
    device_provider: Option<Box<dyn DeviceProvider>>,
    device: Option<Box<dyn Device>>,
    stream: Option<Box<dyn OutputStream>>,
    state: PlaybackState,
    resampler: Option<Resampler>,
    format: Option<FormatInfo>,
    queue: Vec<String>,
    shuffled_queue: Vec<String>,
    shuffle: bool,
    queue_next: usize,
    last_timestamp: u64,
    pending_reset: bool,
}

impl PlaybackThread {
    /// Starts the playback thread and returns the created interface.
    pub fn start<T: PlaybackInterface>() -> T {
        let (commands_tx, commands_rx) = std::sync::mpsc::channel();
        let (events_tx, events_rx) = std::sync::mpsc::channel();

        std::thread::Builder::new()
            .name("playback".to_string())
            .spawn(move || {
                let mut thread = PlaybackThread {
                    commands_rx,
                    events_tx,
                    media_provider: None,
                    device_provider: None,
                    device: None,
                    stream: None,
                    state: PlaybackState::Stopped,
                    resampler: None,
                    format: None,
                    queue: Vec::new(),
                    shuffled_queue: Vec::new(),
                    shuffle: false,
                    queue_next: 0,
                    last_timestamp: u64::MAX,
                    pending_reset: false,
                };

                thread.run();
            })
            .expect("could not start playback thread");

        T::new(commands_tx, events_rx)
    }

    pub fn run(&mut self) {
        // for now just throw in the default Providers and pick the default Device
        // TODO: Add a way to select the Device and MediaProvider
        #[cfg(target_os = "linux")]
        {
            self.device_provider = Some(Box::new(PulseProvider::default()));
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.device_provider = Some(Box::new(CpalProvider::default()));
        }

        self.media_provider = Some(Box::new(SymphoniaProvider::default()));
        self.device = Some(
            self.device_provider
                .as_mut()
                .unwrap()
                .get_default_device()
                .unwrap(),
        );

        let format = self.device.as_ref().unwrap().get_default_format().unwrap();

        info!(
            "Opened device: {:?}, format: {:?}, rate: {}",
            self.device.as_ref().unwrap().get_name(),
            format.sample_type,
            format.sample_rate
        );

        loop {
            self.main_loop();
        }
    }

    pub fn main_loop(&mut self) {
        self.command_intake();

        if self.state == PlaybackState::Playing {
            self.play_audio();
        } else {
            sleep(std::time::Duration::from_millis(10));
        }

        self.broadcast_events();
    }

    pub fn broadcast_events(&mut self) {
        if let Some(provider) = &mut self.media_provider {
            if provider.metadata_updated() {
                // TODO: proper error handling
                let metadata = provider.read_metadata().expect("failed to get metadata");
                self.events_tx
                    .send(PlaybackEvent::MetadataUpdate(Box::new(metadata.clone())))
                    .expect("unable to send event");

                let image = provider.read_image().expect("failed to decode image");
                self.events_tx
                    .send(PlaybackEvent::AlbumArtUpdate(image))
                    .expect("unable to send event");
            }
        }
    }

    pub fn command_intake(&mut self) {
        while let Ok(command) = self.commands_rx.try_recv() {
            match command {
                PlaybackCommand::Play => self.play(),
                PlaybackCommand::Pause => self.pause(),
                PlaybackCommand::Open(v) => self.open(&v),
                PlaybackCommand::Queue(v) => self.queue(&v),
                PlaybackCommand::QueueList(v) => self.queue_list(v),
                PlaybackCommand::Next => self.next(true),
                PlaybackCommand::Previous => self.previous(),
                PlaybackCommand::ClearQueue => self.clear_queue(),
                PlaybackCommand::Jump(v) => self.jump(v),
                PlaybackCommand::Seek(v) => self.seek(v),
                PlaybackCommand::SetVolume(_) => todo!(),
                PlaybackCommand::ReplaceQueue(v) => self.replace_queue(v),
                PlaybackCommand::Stop => self.stop(),
                PlaybackCommand::ToggleShuffle => self.toggle_shuffle(),
            }
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Paused {
            return;
        }

        if self.state == PlaybackState::Playing {
            if let Some(stream) = &mut self.stream {
                stream.pause().expect("unable to pause stream");
            }

            self.state = PlaybackState::Paused;

            self.events_tx
                .send(PlaybackEvent::StateChanged(PlaybackState::Paused))
                .expect("unable to send event");
        }
    }

    pub fn play(&mut self) {
        if self.state == PlaybackState::Playing {
            return;
        }

        if self.state == PlaybackState::Paused {
            if let Some(stream) = &mut self.stream {
                if self.pending_reset {
                    stream.reset().expect("unable to reset stream");
                    self.pending_reset = false;
                }

                stream.play().expect("unable to play stream");
            }

            self.state = PlaybackState::Playing;

            self.events_tx
                .send(PlaybackEvent::StateChanged(PlaybackState::Playing))
                .expect("unable to send event");
        }

        if self.state == PlaybackState::Stopped && !self.queue.is_empty() {
            self.open(&(self.queue[0].clone()));
            self.queue_next = 1;
        }

        // nothing to play, womp womp
    }

    fn open(&mut self, path: &String) {
        info!("Opening: {}", path);
        if self.stream.is_none() {
            // TODO: proper error handling
            // TODO: allow the user to pick a format on supported platforms
            let format = self.device.as_ref().unwrap().get_default_format().unwrap();
            self.stream = Some(self.device.as_mut().unwrap().open_device(format).unwrap());
        }

        if self.state == PlaybackState::Paused {
            self.stream
                .as_mut()
                .unwrap()
                .reset()
                .expect("unable to reset device");
        }

        self.stream
            .as_mut()
            .unwrap()
            .play()
            .expect("unable to play stream");

        // TODO: handle multiple media providers
        if let Some(provider) = &mut self.media_provider {
            // TODO: proper error handling
            self.resampler = None;
            let src = std::fs::File::open(path).expect("failed to open media");
            provider.open(src, None).expect("unable to open file");
            provider.start_playback().expect("unable to start playback");

            self.state = PlaybackState::Playing;
            self.events_tx
                .send(PlaybackEvent::SongChanged(path.clone()))
                .expect("unable to send event");

            if let Ok(duration) = provider.duration_secs() {
                self.events_tx
                    .send(PlaybackEvent::DurationChanged(duration))
                    .expect("unable to send event");
            } else {
                self.events_tx
                    .send(PlaybackEvent::DurationChanged(0))
                    .expect("unable to send event");
            }

            self.update_ts();

            self.events_tx
                .send(PlaybackEvent::StateChanged(PlaybackState::Playing))
                .expect("unable to send event");
        }
    }

    fn next(&mut self, user_initiated: bool) {
        if self.queue_next < self.queue.len() {
            info!("Opening next file in queue");
            let next_path = if self.shuffle {
                self.shuffled_queue[self.queue_next].clone()
            } else {
                self.queue[self.queue_next].clone()
            };
            self.open(&next_path);
            self.queue_next += 1;
        } else if !user_initiated {
            info!("Playback queue is empty, stopping playback");
            self.stop();
        }
    }

    fn previous(&mut self) {
        if self.state == PlaybackState::Stopped && !self.queue.is_empty() {
            let track = if self.shuffle {
                self.shuffled_queue.last().unwrap().clone()
            } else {
                self.queue.last().unwrap().clone()
            };
            self.open(&track);
            self.queue_next = self.queue.len();
        } else if self.queue_next > 1 {
            info!("Opening previous file in queue");
            let prev_path = if self.shuffle {
                self.shuffled_queue[self.queue_next - 2].clone()
            } else {
                self.queue[self.queue_next - 2].clone()
            };
            self.queue_next -= 1;
            debug!("queue_next: {}", self.queue_next);
            self.open(&prev_path);
        }
    }

    fn queue(&mut self, path: &String) {
        info!("Adding file to queue: {}", path);
        let pre_len = self.queue.len();
        self.queue.push(path.clone());

        if self.shuffle {
            self.shuffled_queue.push(path.clone());
        }

        if self.state == PlaybackState::Stopped {
            self.open(path);
            self.queue_next = pre_len + 1;
            self.events_tx
                .send(PlaybackEvent::QueuePositionChanged(pre_len))
                .expect("unable to send event");
        }

        if self.shuffle {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.shuffled_queue.clone()))
                .expect("unable to send event");
        } else {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.queue.clone()))
                .expect("unable to send event");
        }
    }

    fn queue_list(&mut self, mut paths: Vec<String>) {
        info!("Adding files to queue: {:?}", paths);
        let pre_len = self.queue.len();
        let first = paths.first().cloned();

        if self.shuffle {
            let mut shuffled_paths = paths.clone();
            shuffled_paths.shuffle(&mut thread_rng());

            self.shuffled_queue.append(&mut shuffled_paths);
        }

        self.queue.append(&mut paths);

        if self.state == PlaybackState::Stopped {
            if let Some(first) = first {
                self.open(&first);
                self.queue_next = pre_len + 1;
            }
        }

        if self.shuffle {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.shuffled_queue.clone()))
                .expect("unable to send event");
        } else {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.queue.clone()))
                .expect("unable to send event");
        }
    }

    fn update_ts(&mut self) {
        if let Some(provider) = &self.media_provider {
            if let Ok(timestamp) = provider.position_secs() {
                if timestamp == self.last_timestamp {
                    return;
                }

                self.events_tx
                    .send(PlaybackEvent::PositionChanged(timestamp))
                    .expect("unable to send event");

                self.last_timestamp = timestamp;
            }
        }
    }

    fn seek(&mut self, timestamp: f64) {
        if let Some(provider) = &mut self.media_provider {
            provider.seek(timestamp).expect("unable to seek");
            self.pending_reset = true;
            self.update_ts();
        }
    }

    fn jump(&mut self, index: usize) {
        if index < self.queue.len() {
            if self.shuffle {
                self.open(&self.shuffled_queue[index].clone());
            } else {
                self.open(&self.queue[index].clone());
            }
            self.queue_next = index + 1;
        }
    }

    fn replace_queue(&mut self, paths: Vec<String>) {
        info!("Replacing queue with: {:?}", paths);

        if self.shuffle {
            let mut shuffled_paths = paths.clone();
            shuffled_paths.shuffle(&mut thread_rng());

            self.shuffled_queue = shuffled_paths;
        }

        self.queue = paths;
        self.queue_next = 0;
        self.jump(0);

        if self.shuffle {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.shuffled_queue.clone()))
                .expect("unable to send event");
        } else {
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.queue.clone()))
                .expect("unable to send event");
        }
    }

    fn clear_queue(&mut self) {
        self.queue = Vec::new();
        self.queue_next = 0;
        self.events_tx
            .send(PlaybackEvent::QueueUpdated(self.queue.clone()))
            .expect("unable to send event");
    }

    fn stop(&mut self) {
        if let Some(provider) = &mut self.media_provider {
            provider.stop_playback().expect("unable to stop playback");
            provider.close().expect("unable to close media");
        }
        self.state = PlaybackState::Stopped;
        self.events_tx
            .send(PlaybackEvent::StateChanged(PlaybackState::Stopped))
            .expect("unable to send event");
    }

    fn toggle_shuffle(&mut self) {
        if self.shuffle {
            // find the current track in the unshuffled queue
            if self.queue_next > 0 {
                let current = self.shuffled_queue[self.queue_next - 1].clone();
                let index = self.queue.iter().position(|x| x == &current).unwrap();
                self.queue_next = index + 1;
            }

            self.shuffled_queue = Vec::new();
            self.shuffle = false;

            self.events_tx
                .send(PlaybackEvent::ShuffleToggled(false))
                .expect("unable to send event");
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.queue.clone()))
                .expect("unable to send event");
        } else {
            self.shuffled_queue = self.queue.clone();
            let length = self.shuffled_queue.len();
            self.shuffled_queue[self.queue_next..length].shuffle(&mut thread_rng());
            self.shuffle = true;

            self.events_tx
                .send(PlaybackEvent::ShuffleToggled(true))
                .expect("unable to send event");
            self.events_tx
                .send(PlaybackEvent::QueueUpdated(self.shuffled_queue.clone()))
                .expect("unable to send event");
        }
    }

    fn play_audio(&mut self) {
        if let Some(stream) = &mut self.stream {
            if let Some(provider) = &mut self.media_provider {
                if self.resampler.is_none() {
                    // TODO: proper error handling
                    let first_samples = match provider.read_samples() {
                        Ok(samples) => samples,
                        Err(e) => match e {
                            PlaybackReadError::NothingOpen => {
                                panic!("thread state is invalid: no file open")
                            }
                            PlaybackReadError::NeverStarted => {
                                panic!("thread state is invalid: playback never started")
                            }
                            PlaybackReadError::EOF => {
                                info!("EOF, moving to next song");
                                self.next(false);
                                return;
                            }
                            PlaybackReadError::Unknown => return,
                            PlaybackReadError::DecodeFatal => panic!("fatal decoding error"),
                        },
                    };
                    let duration = provider.frame_duration().expect("can't get duration");
                    let device_format = stream.get_current_format().unwrap();

                    self.resampler = Some(Resampler::new(
                        first_samples.rate,
                        device_format.sample_rate,
                        duration,
                        // TODO: support getting channels from the bitmask
                        match device_format.channels {
                            ChannelSpec::Count(v) => v,
                            _ => 2,
                        },
                    ));
                    self.format = Some(device_format.clone());

                    let converted = self
                        .resampler
                        .as_mut()
                        .unwrap()
                        .convert_formats(first_samples, self.format.as_ref().unwrap());

                    stream
                        .submit_frame(converted)
                        .expect("failed to submit frames to stream");

                    self.update_ts();
                } else {
                    let samples = match provider.read_samples() {
                        Ok(samples) => samples,
                        Err(e) => match e {
                            PlaybackReadError::NothingOpen => {
                                panic!("thread state is invalid: no file open")
                            }
                            PlaybackReadError::NeverStarted => {
                                panic!("thread state is invalid: playback never started")
                            }
                            PlaybackReadError::EOF => {
                                info!("EOF, moving to next song");
                                self.next(false);
                                return;
                            }
                            PlaybackReadError::Unknown => return,
                            PlaybackReadError::DecodeFatal => panic!("fatal decoding error"),
                        },
                    };
                    let converted = self
                        .resampler
                        .as_mut()
                        .unwrap()
                        .convert_formats(samples, self.format.as_ref().unwrap());

                    stream
                        .submit_frame(converted)
                        .expect("failed to submit frames to stream");

                    self.update_ts();
                }
            }
        }
    }
}
