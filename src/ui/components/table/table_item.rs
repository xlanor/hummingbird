use std::{marker::PhantomData, sync::Arc};

use gpui::{prelude::FluentBuilder, *};
use tracing::info;

use crate::ui::theme::Theme;

use super::table_data::TableData;

#[derive(Clone)]
pub struct TableItem<T>
where
    T: TableData + 'static,
{
    data: Option<Vec<Option<SharedString>>>,
    image: Option<Arc<RenderImage>>,
    widths: Entity<Vec<f32>>,
    phantom: PhantomData<T>,
}

impl<T> TableItem<T>
where
    T: TableData + 'static,
{
    pub fn new(cx: &mut App, id: T::Identifier, widths: Entity<Vec<f32>>) -> Entity<Self> {
        info!("hi from {:?}", id);

        let row = T::get_row(cx, id).ok().flatten();

        let data = row.as_ref().map(|row| {
            T::get_column_names()
                .iter()
                .map(|v| row.get_column(cx, v))
                .collect()
        });

        let image = row.as_ref().and_then(|row| row.get_image());

        cx.new(|cx| Self {
            data,
            image,
            widths,
            phantom: PhantomData,
        })
    }
}

impl<T> Render for TableItem<T>
where
    T: TableData + 'static,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let mut row = div().w_full().flex();

        if let Some(data) = self.data.as_ref() {
            for (i, column) in data.iter().enumerate() {
                let width = self.widths.read(cx).get(i).cloned().unwrap_or(100.0);
                let column = div()
                    .w(px(width))
                    .h(px(30.0))
                    .px(px(10.0))
                    .py(px(2.0))
                    .text_sm()
                    .flex_shrink_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(i != data.len() - 1, |div| {
                        div.border_r_1().border_color(theme.border_color)
                    })
                    .border_b_1()
                    .border_color(theme.border_color)
                    .when_some(column.clone(), |div, string| div.child(string));

                row = row.child(column);
            }
        }

        row
    }
}
