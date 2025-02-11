mod model;

use std::collections::VecDeque;

use gpui::*;
use model::SearchModel;
use tracing::debug;

use super::{
    components::{input::TextInput, modal::modal},
    global_actions::Search,
    library::ViewSwitchMessage,
    models::Models,
    theme::Theme,
};

pub struct SearchView {
    show: Entity<bool>,
    input: Entity<TextInput>,
    search: Entity<SearchModel>,
    view_switcher: Entity<VecDeque<ViewSwitchMessage>>,
}

impl SearchView {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let show = cx.new(|_| false);
            let show_clone = show.clone();
            let handle = cx.focus_handle();
            let input = TextInput::new(cx, handle, None, None);
            let search = SearchModel::new(cx);

            App::on_action(cx, move |_: &Search, cx| {
                show_clone.update(cx, |m, cx| {
                    *m = true;
                    cx.notify();
                })
            });

            cx.subscribe(&input, |this: &mut SearchView, _, ev, cx| {
                debug!("Input event: {:?}", ev);
                let input = ev.clone();
                cx.update_entity(&this.search, |_, cx| {
                    cx.emit(input);
                })
            })
            .detach();

            cx.subscribe(
                &search,
                |this: &mut SearchView, _, ev: &ViewSwitchMessage, cx| {
                    this.view_switcher.update(cx, |_, cx| {
                        cx.emit(*ev);
                    });
                    this.reset(cx);
                },
            )
            .detach();

            cx.observe(&show, |_, _, cx| {
                cx.notify();
            })
            .detach();

            SearchView {
                view_switcher: cx.global::<Models>().switcher_model.clone(),
                show,
                input,
                search,
            }
        })
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        cx.update_entity(&self.input, |input, cx| {
            input.reset();
            cx.notify();
        });
        cx.update_entity(&self.search, |search, cx| {
            search.set_query("".to_string());
            cx.notify();
        });
        self.show.update(cx, |m, cx| {
            *m = false;
            cx.notify();
        })
    }
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show = self.show.clone();
        let show_read = show.read(cx);
        let theme = cx.global::<Theme>();
        let weak = cx.weak_entity();

        if *show_read {
            modal()
                .on_exit(move |_, cx| {
                    weak.update(cx, |this, cx| {
                        this.reset(cx);
                    })
                    .expect("failed to update search view")
                })
                .child(
                    div()
                        .w(px(500.0))
                        .h(px(400.0))
                        .overflow_hidden()
                        .flex_col()
                        .child(
                            div()
                                .w_full()
                                .p(px(10.0))
                                .line_height(px(14.0))
                                .h(px(36.0))
                                .text_sm()
                                .border_b(px(1.0))
                                .border_color(theme.border_color)
                                .child(self.input.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .w_full()
                                .h_full()
                                // FIXME: weird layout issue, this is a hack
                                // eventually this should be removed
                                .pb(px(36.0))
                                .child(self.search.clone()),
                        ),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
