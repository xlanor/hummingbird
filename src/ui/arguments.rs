use std::path::PathBuf;

use clap::Parser;
use tracing::info;

use crate::playback::interface::GPUIPlaybackInterface;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg()]
    files: Option<Vec<PathBuf>>,
}

pub fn parse_args_and_prepare(interface: &GPUIPlaybackInterface) {
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
                .collect(),
        );
    }
}
