use std::{fs::File, io::BufReader, path::PathBuf, sync::mpsc::channel, time::Duration};

use gpui::{rgb, rgba, AppContext, AsyncAppContext, Context, EventEmitter, Global, Rgba};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use tracing::{error, info, warn};

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Theme {
    pub background_primary: Rgba,
    pub background_secondary: Rgba,
    pub background_tertiary: Rgba,

    pub border_color: Rgba,

    pub album_art_background: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,

    pub nav_button_hover: Rgba,
    pub nav_button_active: Rgba,

    pub playback_button: Rgba,
    pub playback_button_hover: Rgba,
    pub playback_button_active: Rgba,
    pub playback_button_border: Rgba,

    pub window_button: Rgba,
    pub window_button_hover: Rgba,
    pub window_button_active: Rgba,

    pub close_button: Rgba,
    pub close_button_hover: Rgba,
    pub close_button_active: Rgba,

    pub queue_item: Rgba,
    pub queue_item_hover: Rgba,
    pub queue_item_active: Rgba,
    pub queue_item_current: Rgba,

    pub button_primary: Rgba,
    pub button_primary_hover: Rgba,
    pub button_primary_active: Rgba,
    pub button_primary_text: Rgba,

    pub button_secondary: Rgba,
    pub button_secondary_hover: Rgba,
    pub button_secondary_active: Rgba,
    pub button_secondary_text: Rgba,

    pub button_warning: Rgba,
    pub button_warning_hover: Rgba,
    pub button_warning_active: Rgba,
    pub button_warning_text: Rgba,

    pub button_danger: Rgba,
    pub button_danger_hover: Rgba,
    pub button_danger_active: Rgba,
    pub button_danger_text: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        // TODO: Theme for scrubber (when scrubber is rewritten)
        Self {
            background_primary: rgb(0x030712),
            background_secondary: rgb(0x111827),
            background_tertiary: rgb(0x1e293b),

            border_color: rgb(0x1e293b),

            album_art_background: rgb(0x4b5563),
            text: rgb(0xf1f5f9),
            text_secondary: rgb(0xd1d5db),

            nav_button_hover: rgb(0x1e293b),
            nav_button_active: rgb(0x111827),

            playback_button: rgb(0x1f2937),
            playback_button_hover: rgb(0x374151),
            playback_button_active: rgb(0x111827),
            playback_button_border: rgb(0x374151),

            window_button: rgba(0x33415500),
            window_button_hover: rgb(0x334155),
            window_button_active: rgb(0x111827),

            queue_item: rgb(0x1e293b00),
            queue_item_hover: rgb(0x1f2937),
            queue_item_active: rgb(0x030712),
            queue_item_current: rgb(0x1f2937),

            close_button: rgba(0x33415500),
            close_button_hover: rgb(0x991b1b),
            close_button_active: rgb(0x111827),

            button_primary: rgb(0x1e3a8a),
            button_primary_hover: rgb(0x1e40af),
            button_primary_active: rgb(0x172554),
            button_primary_text: rgb(0xeff6ff),

            button_secondary: rgb(0x1f2937),
            button_secondary_hover: rgb(0x334155),
            button_secondary_active: rgb(0x0f172a),
            button_secondary_text: rgb(0xf1f5f9),

            button_warning: rgb(0x854d0e),
            button_warning_hover: rgb(0xa16207),
            button_warning_active: rgb(0x713f12),
            button_warning_text: rgb(0xfefce8),

            button_danger: rgb(0x7f1d1d),
            button_danger_hover: rgb(0x991b1b),
            button_danger_active: rgb(0x450a0a),
            button_danger_text: rgb(0xfef2f2),
        }
    }
}

impl Global for Theme {}

pub fn create_theme(path: &PathBuf) -> Theme {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);

        if let Ok(theme) = serde_json::from_reader(reader) {
            theme
        } else {
            warn!("Theme file exists but it could not be loaded, using default");
            Theme::default()
        }
    } else {
        Theme::default()
    }
}

#[derive(PartialEq, Clone)]
pub struct ThemeEvTransmitter;

impl EventEmitter<Theme> for ThemeEvTransmitter {}

#[allow(dead_code)]
pub struct ThemeWatcher(pub Box<dyn Watcher>);

impl Global for ThemeWatcher {}

pub fn setup_theme(cx: &mut AppContext, path: PathBuf) {
    cx.set_global(create_theme(&path));
    let theme_transmitter = cx.new_model(|_| ThemeEvTransmitter);

    cx.subscribe(&theme_transmitter, |_, theme, cx| {
        cx.set_global(theme.clone());
        cx.refresh();
    })
    .detach();

    let (tx, rx) = channel::<notify::Result<Event>>();

    let watcher = notify::recommended_watcher(tx);

    if let Ok(mut watcher) = watcher {
        if let Err(e) = watcher.watch(path.parent().unwrap(), RecursiveMode::Recursive) {
            warn!("failed to watch settings directory: {:?}", e);
        }

        cx.spawn(|mut cx: AsyncAppContext| async move {
            loop {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        Ok(v) => {
                            if v.paths.iter().find(|t| t.ends_with("theme.json")).is_some() {
                                match v.kind {
                                    notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                        info!("Theme changed, updating...");
                                        let theme = create_theme(&path);
                                        theme_transmitter
                                            .update(&mut cx, move |_, m| {
                                                m.emit(theme);
                                            })
                                            .expect("could not send theme to main thread");
                                    }
                                    notify::EventKind::Remove(_) => {
                                        info!("Theme file removed, resetting to default...");
                                        theme_transmitter
                                            .update(&mut cx, |_, m| {
                                                m.emit(Theme::default());
                                            })
                                            .expect("could not send theme to main thread");
                                    }
                                    _ => (),
                                }
                            }
                        }
                        Err(e) => error!("error occured while watching theme.json: {:?}", e),
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
        })
        .detach();

        // store the watcher in a global so it doesn't go out of scope
        let tw = ThemeWatcher(Box::new(watcher));
        cx.set_global(tw);
    } else if let Err(e) = watcher {
        warn!("failed to watch settings directory: {:?}", e);
    }
}
