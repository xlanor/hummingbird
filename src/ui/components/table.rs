pub mod table_data;
mod table_item;

use std::{collections::VecDeque, marker::PhantomData, rc::Rc, sync::Arc};

use ahash::AHashMap;
use gpui::{prelude::FluentBuilder, *};
use table_data::{TableData, TableSort};
use table_item::TableItem;
use tracing::{info, warn};

use crate::{
    library::db::LibraryAccess,
    ui::{
        library::ViewSwitchMessage,
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

type RowMap<T> = AHashMap<usize, Entity<TableItem<T>>>;

#[derive(Clone)]
pub struct Table<T>
where
    T: TableData + 'static,
{
    columns: &'static [&'static str],
    widths: Entity<Vec<f32>>,
    views: Entity<RowMap<T>>,
    render_counter: Entity<usize>,
    list_state: ListState,
    search_method: Entity<Option<TableSort>>,
    view_switcher: Entity<VecDeque<ViewSwitchMessage>>,
}

impl<T> Table<T>
where
    T: TableData + 'static,
{
    pub fn new(cx: &mut App, view_switcher: Entity<VecDeque<ViewSwitchMessage>>) -> Entity<Self> {
        cx.new(|cx| {
            let widths = cx.new(|_| T::default_column_widths());
            let views = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);
            let search_method = cx.new(|_| None);

            let list_state = Self::make_list_state(
                cx,
                views.clone(),
                render_counter.clone(),
                view_switcher.clone(),
                &search_method,
                widths.clone(),
            );

            Self {
                columns: T::get_column_names(),
                widths,
                views,
                render_counter,
                list_state,
                search_method,
                view_switcher,
            }
        })
    }

    fn regenerate_list_state(&mut self, cx: &mut Context<'_, Self>) {
        let curr_scroll = self.list_state.logical_scroll_top();
        self.views = cx.new(|_| AHashMap::new());
        self.render_counter = cx.new(|_| 0);

        self.list_state = Self::make_list_state(
            cx,
            self.views.clone(),
            self.render_counter.clone(),
            self.view_switcher.clone(),
            &self.search_method,
            self.widths.clone(),
        );

        self.list_state.scroll_to(curr_scroll);

        cx.notify();
    }

    fn make_list_state(
        cx: &mut Context<'_, Self>,
        views: Entity<RowMap<T>>,
        render_counter: Entity<usize>,
        view_switcher: Entity<VecDeque<ViewSwitchMessage>>,
        search_method: &Entity<Option<TableSort>>,
        widths: Entity<Vec<f32>>,
    ) -> ListState {
        let sort_method = *search_method.read(cx);
        let Ok(rows) = T::get_rows(cx, sort_method) else {
            warn!("Failed to get rows");
            return ListState::new(0, ListAlignment::Top, px(64.0), move |_, _, _| {
                div().into_any_element()
            });
        };

        info!("got {} rows", rows.len());

        let idents_rc = Rc::new(rows);

        ListState::new(
            idents_rc.len(),
            ListAlignment::Top,
            px(300.0),
            move |idx, _, cx| {
                let idents_rc = idents_rc.clone();

                prune_views(&views, &render_counter, idx, cx);
                div()
                    .w_full()
                    .child(create_or_retrieve_view(
                        &views,
                        idx,
                        |cx| TableItem::new(cx, idents_rc[idx].clone(), widths.clone()),
                        cx,
                    ))
                    .into_any_element()
            },
        )
    }
}

impl<T> Render for Table<T>
where
    T: TableData + 'static,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let mut header = div().w_full().flex();
        let theme = cx.global::<Theme>();

        if T::has_images() {
            header = header.child(
                div()
                    .w(px(53.0))
                    .h(px(36.0))
                    .pl(px(27.0))
                    .pr(px(10.0))
                    .py(px(2.0))
                    .text_sm()
                    .flex_shrink_0()
                    .text_ellipsis()
                    .border_r_1()
                    .border_color(theme.border_color)
                    .border_b_1()
                    .border_color(theme.border_color),
            );
        }

        for (i, column) in self.columns.iter().enumerate() {
            let width = self.widths.read(cx)[i];
            header = header.child(
                div()
                    .w(px(width))
                    .when(T::has_images(), |div| {
                        div.h(px(36.0)).px(px(12.0)).py(px(5.0))
                    })
                    .when(!T::has_images(), |div| {
                        div.h(px(30.0))
                            .px(px(10.0))
                            .py(px(2.0))
                            .when(i == 0, |div| div.pl(px(27.0)))
                    })
                    .text_sm()
                    .flex_shrink_0()
                    .when(i != self.columns.len() - 1, |div| {
                        div.border_r_1().border_color(theme.border_color)
                    })
                    .border_b_1()
                    .border_color(theme.border_color)
                    .font_weight(FontWeight::BOLD)
                    .child(SharedString::new_static(column)),
            );
        }

        div()
            .id(T::get_table_name())
            .overflow_x_scroll()
            .w_full()
            .h_full()
            .child(
                div()
                    .w_full()
                    .pb(px(11.0))
                    .px(px(24.0))
                    .line_height(px(26.0))
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(26.0))
                    .child(T::get_table_name()),
            )
            .child(header)
            .child(list(self.list_state.clone()).w_full().h_full())
    }
}
