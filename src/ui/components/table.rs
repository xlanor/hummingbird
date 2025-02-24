pub mod table_data;

use std::marker::PhantomData;

use gpui::{prelude::FluentBuilder, *};
use table_data::TableData;

use crate::ui::theme::Theme;

#[derive(Clone)]
pub struct Table<T>
where
    T: TableData + 'static,
{
    phantom: PhantomData<T>,
    columns: &'static [&'static str],
    widths: Entity<Vec<f32>>,
}

impl<T> Table<T>
where
    T: TableData + 'static,
{
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            phantom: PhantomData,
            columns: T::get_column_names(),
            widths: cx.new(|_| T::default_column_widths()),
        })
    }
}

impl<T> Render for Table<T>
where
    T: TableData + 'static,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let mut header = div().w_full().flex();
        let theme = cx.global::<Theme>();

        for (i, column) in self.columns.iter().enumerate() {
            let width = self.widths.read(cx)[i];
            header = header.child(
                div()
                    .w(px(width))
                    .h(px(30.0))
                    .px(px(10.0))
                    .py(px(2.0))
                    .text_sm()
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
            .overflow_scroll()
            .w_full()
            .child(header)
    }
}
