use std::{
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use ahash::AHashMap;
use gpui::*;
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Nucleo, Utf32String,
};
use smallvec::smallvec;
use tracing::debug;

use crate::library::db::LibraryAccess;

pub struct SearchModel {
    query: String,
    matcher: Nucleo<(u32, String)>,
}

impl SearchModel {
    pub fn new(cx: &mut App) -> Entity<SearchModel> {
        cx.new(|cx| {
            let albums = cx
                .list_albums_search()
                .expect("could not retrieve albums from db");

            let config = Config::DEFAULT;

            let (rx, tx) = channel();

            let notify = Arc::new(move || {
                rx.send(()).unwrap();
            });

            cx.spawn(|weak, mut cx| async move {
                loop {
                    while tx.try_recv().is_ok() {
                        weak.update(&mut cx, |_: &mut SearchModel, cx| {
                            cx.notify();
                        })
                        .expect("unable to update weak search model");
                    }

                    weak.update(&mut cx, |t: &mut SearchModel, _| {
                        t.tick();
                    })
                    .expect("unable to update weak search model");

                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
            })
            .detach();

            cx.subscribe(&cx.entity(), |this, _, ev, _| {
                this.set_query(ev.clone());
            })
            .detach();

            let matcher = Nucleo::new(config, notify, None, 1);
            let injector = matcher.injector();

            for album in albums {
                injector.push(album, |v, dest| {
                    dest[0] = Utf32String::from(v.1.clone());
                });
            }

            SearchModel {
                query: String::new(),
                matcher,
            }
        })
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );
    }

    pub fn tick(&mut self) {
        self.matcher.tick(10);
    }

    pub fn get_matches(&self) -> Vec<(u32, String)> {
        let snapshot = self.matcher.snapshot();
        snapshot
            .matched_items(..100.min(snapshot.matched_item_count()))
            .map(|item| item.data.clone())
            .collect()
    }
}

impl EventEmitter<String> for SearchModel {}

impl Render for SearchModel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        debug!("Search results: {:?}", self.get_matches());

        div()
    }
}
