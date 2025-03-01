use std::path::PathBuf;

use clap::Parser;
use gpui::App;
use tracing::info;

use crate::playback::{interface::GPUIPlaybackInterface, queue::QueueItemData};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg()]
    files: Option<Vec<PathBuf>>,
}

pub fn parse_args_and_prepare(cx: &mut App, interface: &GPUIPlaybackInterface) {
    let args = Args::parse();

    if let Some(files) = args.files {
        info!("Queueing files found in arguments: {:?}", files);

        interface.queue_list(
            files
                .iter()
                .map(|v| {
                    v.clone()
                        .into_os_string()
                        .into_string()
                        .expect("Invalid path")
                })
                .enumerate()
                // Fix: `idx` is passed as track id, which might not exist in db
                .map(|(idx, v)| QueueItemData::new(cx, v, idx as i64, None))
                .collect(),
        );
    }
}
