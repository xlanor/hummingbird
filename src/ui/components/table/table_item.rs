use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

use super::{
    OnSelectHandler,
    table_data::{Column, TABLE_IMAGE_COLUMN_WIDTH, TABLE_MAX_WIDTH, TableData},
};
use crate::ui::theme::Theme;

/// Calculates the extra width to add to the final column to fill available space.
/// This is required so that the table does not just appear to "end" before it logically should.
fn calculate_final_column_extra_width<C: Column>(
    columns: &IndexMap<C, f32, FxBuildHasher>,
    has_images: bool,
) -> f32 {
    let total_width: f32 = columns.values().sum();
    let available_width = if has_images {
        TABLE_MAX_WIDTH - TABLE_IMAGE_COLUMN_WIDTH
    } else {
        TABLE_MAX_WIDTH
    };
    (available_width - total_width).max(0.0)
}

#[derive(Clone)]
pub struct TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    data: Option<Vec<Option<SharedString>>>,
    columns: Arc<IndexMap<C, f32, FxBuildHasher>>,
    on_select: Option<OnSelectHandler<T, C>>,
    row: Option<Arc<T>>,
    id: Option<ElementId>,
    image_path: Option<SharedString>,
}

impl<T, C> TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    pub fn new(
        cx: &mut App,
        id: T::Identifier,
        columns: &Entity<Arc<IndexMap<C, f32, FxBuildHasher>>>,
        on_select: Option<OnSelectHandler<T, C>>,
    ) -> Entity<Self> {
        let row = T::get_row(cx, id).ok().flatten();

        let id = row.as_ref().map(|row| row.get_element_id().into());

        let columns_read = columns.read(cx).clone();

        let data = row.clone().map(|row| {
            let keys = columns_read.keys();

            keys.into_iter().map(|v| row.get_column(cx, *v)).collect()
        });

        let image_path = row.as_ref().and_then(|row| row.get_image_path());

        cx.new(|cx| {
            cx.observe(columns, |this: &mut TableItem<T, C>, m, cx| {
                this.columns = m.read(cx).clone();

                this.data = this.row.clone().map(|row| {
                    let keys = this.columns.keys();

                    keys.into_iter().map(|v| row.get_column(cx, *v)).collect()
                });

                cx.notify();
            })
            .detach();

            Self {
                data,
                image_path,
                columns: columns_read,
                on_select,
                id,
                row,
            }
        })
    }
}

impl<T, C> Render for TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let row_data = self.row.clone();
        let mut row = div()
            .w_full()
            .flex()
            .id(self.id.clone().unwrap_or("bad".into()))
            .when_some(self.on_select.clone(), move |div, on_select| {
                div.on_click(move |_, _, cx| {
                    let id = row_data.as_ref().unwrap().get_table_id();
                    on_select(cx, &id)
                })
                .cursor_pointer()
                .hover(|this| this.bg(theme.nav_button_hover))
                .active(|this| this.bg(theme.nav_button_active))
            });

        if T::has_images() {
            row = row.child(
                div()
                    .w(px(TABLE_IMAGE_COLUMN_WIDTH))
                    .h(px(36.0))
                    .text_sm()
                    .pl(px(11.0))
                    .flex_shrink_0()
                    .text_ellipsis()
                    //.border_r_1()
                    .border_color(theme.border_color)
                    .border_b_1()
                    .border_color(theme.border_color)
                    .flex()
                    .child(
                        div()
                            .m_auto()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded(px(3.0))
                            .bg(theme.album_art_background)
                            .when_some(self.image_path.clone(), |div, image| {
                                div.child(img(image).w(px(22.0)).h(px(22.0)).rounded(px(3.0)))
                            }),
                    ),
            );
        }

        if let Some(data) = self.data.as_ref() {
            let extra_width = calculate_final_column_extra_width(&self.columns, T::has_images());
            let column_count = self.columns.len();

            for (i, column_data) in data.iter().enumerate() {
                let col = self
                    .columns
                    .get_index(i)
                    .expect("data references column outside of viewed table");
                let is_last = i == column_count - 1;
                let base_width = *col.1;
                let width = if is_last {
                    base_width + extra_width
                } else {
                    base_width
                };
                let monospace = T::column_monospace(*col.0);
                row = row.child(
                    div()
                        .w(px(width))
                        .h(px(36.0))
                        .px(px(12.0))
                        .py(px(6.0))
                        .when(!T::has_images() && i == 0, |div| div.pl(px(21.0)))
                        .when(monospace, |div| div.font_family("Roboto Mono"))
                        .text_sm()
                        .flex_shrink_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .border_b_1()
                        .border_color(theme.border_color)
                        .when_some(column_data.clone(), |div, string| div.child(string)),
                );
            }
        }

        row
    }
}
