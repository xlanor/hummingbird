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
                .map(|path| QueueItemData::new(cx, path.clone(), None, None))
                .collect(),
        );
    }
}
