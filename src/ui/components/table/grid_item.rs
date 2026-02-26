use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};

use super::{
    OnSelectHandler,
    table_data::{Column, GridContext, TableData, TableDragData},
};
use crate::ui::{
    components::drag_drop::{AlbumDragData, DragPreview, TrackDragData},
    theme::Theme,
};

#[derive(Clone)]
pub struct GridItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    row: Arc<T>,
    id: ElementId,
    image_path: Option<SharedString>,
    primary_text: SharedString,
    secondary_text: Option<SharedString>,
    on_select: Option<OnSelectHandler<T, C>>,
}

impl<T, C> GridItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    pub fn new(
        cx: &mut App,
        id: T::Identifier,
        on_select: Option<OnSelectHandler<T, C>>,
        context: GridContext,
    ) -> Option<Entity<Self>> {
        let row = T::get_row(cx, id.clone()).ok().flatten()?;

        let element_id = row.get_element_id().into();
        let image_path = row.get_full_image_path().or_else(|| row.get_image_path());

        let grid_content = row.get_grid_content_for(cx, context);
        let (primary_text, secondary_text) = grid_content.unwrap_or(("".into(), None));

        Some(cx.new(|_| Self {
            row,
            id: element_id,
            image_path,
            primary_text,
            secondary_text,
            on_select,
        }))
    }
}

impl<T, C> Render for GridItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let row_data = self.row.clone();

        let drag_data = self.row.get_drag_data();

        let mut container = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .p(px(8.0))
            .rounded_lg()
            .id(self.id.clone())
            .when_some(self.on_select.clone(), move |div, on_select| {
                div.on_click(move |_, _, cx| {
                    let id = row_data.get_table_id();
                    on_select(cx, &id)
                })
                .cursor_pointer()
                .hover(|this| this.bg(theme.nav_button_hover))
                .active(|this| this.bg(theme.nav_button_active))
            });

        container = match drag_data {
            Some(TableDragData::Track(track_data)) => {
                let display_name = track_data.display_name.clone();
                container
                    .on_drag(track_data, move |_, _, _, cx| {
                        DragPreview::new(cx, display_name.clone())
                    })
                    .drag_over::<TrackDragData>(|style, _, _, _| style.bg(gpui::rgba(0x88888822)))
            }
            Some(TableDragData::Album(album_data)) => {
                let display_name = album_data.display_name.clone();
                container
                    .on_drag(album_data, move |_, _, _, cx| {
                        DragPreview::new(cx, display_name.clone())
                    })
                    .drag_over::<AlbumDragData>(|style, _, _, _| style.bg(gpui::rgba(0x88888822)))
            }
            None => container,
        };

        let mut img_container = div()
            .w_full()
            .flex_1()
            .rounded(px(6.0))
            .bg(theme.album_art_background)
            .overflow_hidden();

        if let Some(image) = self.image_path.clone() {
            img_container = img_container.child(
                img(image)
                    .w_full()
                    .h_full()
                    .rounded(px(6.0))
                    .object_fit(ObjectFit::Fill),
            );
        }

        container
            .child(img_container)
            .child(
                div()
                    .mt(px(8.0))
                    .w_full()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_ellipsis()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(self.primary_text.clone()),
            )
            .when_some(self.secondary_text.clone(), |this, secondary| {
                this.child(
                    gpui::div()
                        .w_full()
                        .text_xs()
                        .text_color(theme.text_secondary)
                        .text_ellipsis()
                        .overflow_hidden()
                        .child(secondary),
                )
            })
    }
}
