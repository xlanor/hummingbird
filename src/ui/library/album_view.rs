use std::{collections::VecDeque, rc::Rc};

use gpui::*;

use crate::{
    library::{
        scan::ScanEvent,
        types::{Album, table::AlbumColumn},
    },
    ui::{
        components::table::{Table, TableEvent, table_data::TABLE_MAX_WIDTH},
        models::Models,
    },
};

use super::ViewSwitchMessage;

#[derive(Clone)]
pub struct AlbumView {
    table: Entity<Table<Album, AlbumColumn>>,
}

impl AlbumView {
    pub(super) fn new(
        cx: &mut App,
        view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
        initial_scroll_offset: Option<f32>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.global::<Models>().scan_state.clone();

            let table_settings = cx.global::<Models>().table_settings.clone();
            let initial_settings = table_settings
                .read(cx)
                .get(Table::<Album, AlbumColumn>::get_table_name())
                .cloned();

            let handler = Rc::new(move |cx: &mut App, id: &(u32, String)| {
                view_switch_model
                    .update(cx, |_, cx| cx.emit(ViewSwitchMessage::Release(id.0 as i64)))
            });

            let table = Table::new(
                cx,
                Some(handler),
                initial_scroll_offset,
                initial_settings.as_ref(),
            );

            let table_clone = table.clone();

            cx.observe(&state, move |_: &mut AlbumView, e, cx| {
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

            AlbumView { table }
        })
    }

    pub fn get_scroll_offset(&self, cx: &App) -> f32 {
        self.table.read(cx).get_scroll_offset()
    }
}

impl Render for AlbumView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .max_w(px(TABLE_MAX_WIDTH))
            .pt(px(10.0))
            .pb(px(0.0))
            .child(self.table.clone())
    }
}
