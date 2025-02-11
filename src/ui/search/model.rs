use std::{
    rc::Rc,
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use ahash::AHashMap;
use gpui::*;
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Nucleo, Utf32String,
};
use prelude::FluentBuilder;
use smallvec::smallvec;
use tracing::debug;

use crate::{
    library::{
        db::{AlbumMethod, LibraryAccess},
        types::Album,
    },
    ui::{
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

pub struct SearchModel {
    query: String,
    matcher: Nucleo<(u32, String)>,
    list_state: ListState,
    views_model: Entity<AHashMap<usize, Entity<AlbumSearchResult>>>,
    last_match: Vec<(u32, String)>,
    render_counter: Entity<usize>,
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
                debug!("Resending");
                rx.send(()).unwrap();
            });

            let views_model = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);

            cx.spawn(|weak, mut cx| async move {
                loop {
                    let mut did_regenerate = false;

                    while tx.try_recv().is_ok() {
                        did_regenerate = true;
                        weak.update(&mut cx, |this: &mut SearchModel, cx| {
                            debug!("Received notification, regenerating list state");
                            this.regenerate_list_state(cx);
                            cx.notify();
                        })
                        .expect("unable to update weak search model");
                    }

                    weak.update(&mut cx, |this: &mut SearchModel, cx| {
                        if !did_regenerate {
                            let matches = this.get_matches();
                            if matches != this.last_match {
                                this.last_match = matches;
                                this.regenerate_list_state(cx);
                                cx.notify();
                            }
                        }
                        this.tick();
                    })
                    .expect("unable to update weak search model");

                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
            })
            .detach();

            cx.subscribe(&cx.entity(), |this, _, ev, cx| {
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
                views_model: views_model.clone(),
                render_counter: render_counter.clone(),
                list_state: Self::make_list_state(None, views_model, render_counter),
                last_match: Vec::new(),
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

    fn tick(&mut self) {
        self.matcher.tick(10);
    }

    fn get_matches(&self) -> Vec<(u32, String)> {
        let snapshot = self.matcher.snapshot();
        snapshot
            .matched_items(..100.min(snapshot.matched_item_count()))
            .map(|item| item.data.clone())
            .collect()
    }

    fn regenerate_list_state<V: 'static>(&mut self, cx: &mut Context<V>) {
        debug!("Regenerating list state");
        let curr_scroll = self.list_state.logical_scroll_top();
        let album_ids = self.get_matches();
        debug!("Album IDs: {:?}", album_ids);
        self.views_model = cx.new(|_| AHashMap::new());
        self.render_counter = cx.new(|_| 0);

        self.list_state = SearchModel::make_list_state(
            Some(album_ids),
            self.views_model.clone(),
            self.render_counter.clone(),
        );

        self.list_state.scroll_to(curr_scroll);

        cx.notify();
    }

    fn make_list_state(
        album_ids: Option<Vec<(u32, String)>>,
        views_model: Entity<AHashMap<usize, Entity<AlbumSearchResult>>>,
        render_counter: Entity<usize>,
    ) -> ListState {
        match album_ids {
            Some(album_ids) => {
                let album_ids_copy = Rc::new(album_ids.clone());

                ListState::new(
                    album_ids.len(),
                    ListAlignment::Top,
                    px(300.0),
                    move |idx, _, cx| {
                        let album_ids = album_ids_copy.clone();

                        prune_views(views_model.clone(), render_counter.clone(), idx, cx);
                        // TODO: error handling
                        div()
                            .w_full()
                            .child(create_or_retrieve_view(
                                views_model.clone(),
                                idx,
                                move |cx| AlbumSearchResult::new(cx, album_ids[idx].0 as i64),
                                cx,
                            ))
                            .into_any_element()
                    },
                )
            }
            None => ListState::new(0, ListAlignment::Top, px(64.0), move |_, _, _| {
                div().into_any_element()
            }),
        }
    }
}

impl EventEmitter<String> for SearchModel {}

impl Render for SearchModel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .flex()
            .p(px(3.0))
            .child(list(self.list_state.clone()).gap(px(5.0)).w_full().h_full())
    }
}

struct AlbumSearchResult {
    album: Option<Arc<Album>>,
    artist: Option<Arc<String>>,
}

impl AlbumSearchResult {
    fn new(cx: &mut App, id: i64) -> Entity<AlbumSearchResult> {
        cx.new(|cx| {
            let album = cx.get_album_by_id(id, AlbumMethod::UncachedThumb).ok();

            let artist = album
                .as_ref()
                .and_then(|album| cx.get_artist_name_by_id(album.artist_id).ok());

            AlbumSearchResult { album, artist }
        })
    }
}

impl Render for AlbumSearchResult {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if let Some(album) = self.album.as_ref() {
            div()
                .p(px(7.0))
                .flex()
                .cursor_pointer()
                .id(("searchresult", album.id as u64))
                .hover(|this| this.bg(theme.palette_item_hover))
                .active(|this| this.bg(theme.palette_item_active))
                .rounded(px(4.0))
                .child(
                    div()
                        .rounded(px(2.0))
                        .bg(theme.album_art_background)
                        .shadow_sm()
                        .w(px(20.0))
                        .h(px(20.0))
                        .flex_shrink_0()
                        .when(album.thumb.is_some(), |div| {
                            div.child(
                                img(album.thumb.clone().unwrap().0)
                                    .w(px(20.0))
                                    .h(px(20.0))
                                    .rounded(px(2.0)),
                            )
                        }),
                )
                .child(
                    div()
                        .pl(px(8.0))
                        .mt(px(3.0))
                        .line_height(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_sm()
                        .child(album.title.clone()),
                )
                .when_some(self.artist.as_ref(), |this, name| {
                    this.child(
                        div()
                            .pl(px(8.0))
                            .mt(px(3.0))
                            .line_height(px(14.0))
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(format!("({})", name)),
                    )
                })
        } else {
            debug!("Album not found");
            div().id("badresult")
        }
    }
}
