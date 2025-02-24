use std::sync::Arc;

use gpui::{App, RenderImage, SharedString};

use crate::library::types::Thumbnail;

pub struct TableSort {
    pub column: &'static str,
    pub ascending: bool,
}

// The TableData trait defines the interface for retrieving, sorting, and listing data for a table.
// Implementing this trait allows a table to display data in a structured manner.
pub trait TableData: Sized {
    type Identifier;

    fn get_table_name() -> &'static str;
    fn get_column_names() -> &'static [&'static str];
    fn get_rows(cx: &mut App, sort: Option<TableSort>) -> anyhow::Result<Vec<Self::Identifier>>;
    fn get_row(cx: &mut App, id: Self::Identifier) -> anyhow::Result<Option<Arc<Self>>>;
    fn get_column(&self, cx: &mut App, column: &'static str) -> Option<SharedString>;
    fn get_image(&self) -> Option<Arc<RenderImage>>;
}
