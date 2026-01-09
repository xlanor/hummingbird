use std::{collections::VecDeque, rc::Rc};

use gpui::*;

use crate::{
    library::{
        scan::ScanEvent,
        types::{Track, table::TrackColumn},
    },
    ui::{
        components::table::{Table, TableEvent},
        models::Models,
    },
};

use super::ViewSwitchMessage;

#[derive(Clone)]
pub struct TrackView {
    table: Entity<Table<Track, TrackColumn>>,
}

impl TrackView {
    pub(super) fn new(
        cx: &mut App,
        _view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
        initial_scroll_offset: Option<f32>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.global::<Models>().scan_state.clone();

            let handler = Rc::new(move |_cx: &mut App, _id: &(i64, String)| {
                // TODO: Implement track selection logic
            });

            let table = Table::new(cx, Some(handler), initial_scroll_offset);

            let table_clone = table.clone();

            cx.observe(&state, move |_: &mut TrackView, e, cx| {
                let value = e.read(cx);
                match value {
                    ScanEvent::ScanCompleteIdle => {
                        table_clone.update(cx, |_, cx| cx.emit(TableEvent::NewRows));
                    }
                    ScanEvent::ScanProgress { current, .. } => {
                        if current % 100 == 0 {
                            table_clone.update(cx, |_, cx| cx.emit(TableEvent::NewRows));
                        }
                    }
                    _ => {}
                }
            })
            .detach();

            TrackView { table }
        })
    }

    pub fn get_scroll_offset(&self, cx: &App) -> f32 {
        self.table.read(cx).get_scroll_offset()
    }
}

impl Render for TrackView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .max_w(px(1000.0))
            .pt(px(10.0))
            .pb(px(0.0))
            .child(self.table.clone())
    }
}
