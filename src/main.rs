mod data;
mod devices;
mod library;
mod media;
mod playback;
mod services;
mod settings;
mod ui;
mod util;

#[async_std::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting application");
    crate::ui::app::run().await;
}
