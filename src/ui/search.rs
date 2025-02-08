mod model;

use gpui::*;

use super::{components::modal::modal, global_actions::Search};

pub struct SearchView {
    show: Entity<bool>,
}

impl SearchView {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let show = cx.new(|_| false);
            let show_clone = show.clone();

            App::on_action(cx, move |_: &Search, cx| {
                show_clone.update(cx, |m, cx| {
                    *m = true;
                    cx.notify();
                })
            });

            cx.observe(&show, |_, _, cx| {
                cx.notify();
            })
            .detach();

            SearchView { show }
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
                .child("Hello")
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
