mod model;

use gpui::*;
use model::SearchModel;
use tracing::debug;

use super::{
    components::{input::TextInput, modal::modal},
    global_actions::Search,
};

pub struct SearchView {
    show: Entity<bool>,
    input: Entity<TextInput>,
    search: Entity<SearchModel>,
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

            cx.observe(&show, |_, _, cx| {
                cx.notify();
            })
            .detach();

            SearchView {
                show,
                input,
                search,
            }
        })
    }
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show = self.show.clone();
        let show_read = show.read(cx);

        if *show_read {
            modal()
                .on_exit(move |_, cx| {
                    show.update(cx, |m, cx| {
                        *m = false;
                        cx.notify();
                    })
                })
                .child(
                    div()
                        .p(px(10.0))
                        .line_height(px(16.0))
                        .w(px(500.0))
                        .child(self.input.clone())
                        .child(self.search.clone()),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
