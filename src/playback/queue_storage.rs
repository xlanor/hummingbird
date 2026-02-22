use std::{io::BufReader, path::PathBuf};

use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::mpsc::UnboundedReceiver};
use tracing::error;

use crate::playback::queue::QueueItemData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueueStorageEvent {
    Add {
        item: QueueItemData,
    },
    AddList {
        items: Vec<QueueItemData>,
    },
    InsertAt {
        item: QueueItemData,
        position: usize,
    },
    InsertListAt {
        items: Vec<QueueItemData>,
        position: usize,
    },
    Remove {
        index: usize,
    },
    Move {
        from: usize,
        to: usize,
    },
    Replace {
        items: Vec<QueueItemData>,
    },
    Clear,
}

pub struct QueueStorageWorker {
    file_path: PathBuf,
    rx: UnboundedReceiver<QueueStorageEvent>,
}

impl QueueStorageWorker {
    pub fn new(file_path: PathBuf, rx: UnboundedReceiver<QueueStorageEvent>) -> Self {
        Self { file_path, rx }
    }

    pub async fn run(mut self) {
        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                error!("Unable to open queue.jsonl for writing: {}", e);
                return;
            }
        };

        while let Some(event) = self.rx.recv().await {
            match &event {
                QueueStorageEvent::Clear | QueueStorageEvent::Replace { .. } => {
                    // Truncate file and write only this event
                    file = match OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&self.file_path)
                        .await
                    {
                        Ok(f) => f,
                        Err(e) => {
                            error!("Unable to truncate queue.jsonl: {}", e);
                            continue;
                        }
                    };
                }
                _ => {}
            }

            match serde_json::to_string(&event) {
                Ok(mut json_str) => {
                    json_str.push('\n');
                    if let Err(e) = file.write_all(json_str.as_bytes()).await {
                        error!("Failed to write to queue.jsonl: {}", e);
                    }

                    // re-open in append mode if we just truncated
                    match &event {
                        QueueStorageEvent::Clear | QueueStorageEvent::Replace { .. } => {
                            file = match OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&self.file_path)
                                .await
                            {
                                Ok(f) => f,
                                Err(e) => {
                                    error!("Unable to reopen queue.jsonl for appending: {}", e);
                                    continue;
                                }
                            };
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!("Failed to serialize QueueStorageEvent: {}", e);
                }
            }
        }
    }

    pub fn load(file_path: &PathBuf) -> Vec<QueueItemData> {
        let mut queue = Vec::new();
        let file = match std::fs::File::open(file_path) {
            Ok(f) => f,
            Err(_) => return queue, // File probably doesn't exist yet
        };

        let reader = BufReader::new(file);
        let deserializer = serde_json::Deserializer::from_reader(reader);

        for event in deserializer.into_iter::<QueueStorageEvent>() {
            match event {
                Ok(QueueStorageEvent::Add { item }) => {
                    queue.push(item);
                }
                Ok(QueueStorageEvent::AddList { items }) => {
                    queue.extend(items);
                }
                Ok(QueueStorageEvent::InsertAt { item, position }) => {
                    if position <= queue.len() {
                        queue.insert(position, item);
                    } else {
                        queue.push(item);
                    }
                }
                Ok(QueueStorageEvent::InsertListAt { items, position }) => {
                    if position <= queue.len() {
                        queue.splice(position..position, items);
                    } else {
                        queue.extend(items);
                    }
                }
                Ok(QueueStorageEvent::Remove { index }) => {
                    if index < queue.len() {
                        queue.remove(index);
                    }
                }
                Ok(QueueStorageEvent::Move { from, to }) => {
                    if from < queue.len() && to < queue.len() {
                        let item = queue.remove(from);
                        queue.insert(to, item);
                    }
                }
                Ok(QueueStorageEvent::Replace { items }) => {
                    queue = items;
                }
                Ok(QueueStorageEvent::Clear) => {
                    queue.clear();
                }
                Err(e) => {
                    error!("Error deserializing QueueStorageEvent: {}", e);
                    // consider it corrupted from here on, breaking might be safer to prevent
                    // inconsistent queue state vs just logging and continuing.
                    break;
                }
            }
        }

        queue
    }
}
