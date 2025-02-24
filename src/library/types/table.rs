use std::sync::Arc;

use gpui::{App, RenderImage, SharedString};

use super::Album;
use crate::{
    library::db::{AlbumMethod, AlbumSortMethod, LibraryAccess},
    ui::components::table::table_data::{TableData, TableSort},
};

impl TableData for Album {
    type Identifier = (u32, String);

    fn get_table_name() -> &'static str {
        "Albums"
    }

    fn get_column_names() -> &'static [&'static str] {
        &["Title", "Artist", "Release Date", "Label", "Catalog Number"]
    }

    fn get_rows(
        cx: &mut gpui::App,
        sort: Option<TableSort>,
    ) -> anyhow::Result<Vec<Self::Identifier>> {
        let sort_method = match sort {
            Some(TableSort {
                column: "Title",
                ascending: true,
            }) => AlbumSortMethod::TitleAsc,
            Some(TableSort {
                column: "Title",
                ascending: false,
            }) => AlbumSortMethod::TitleDesc,
            Some(TableSort {
                column: "Artist",
                ascending: true,
            }) => AlbumSortMethod::ArtistAsc,
            Some(TableSort {
                column: "Artist",
                ascending: false,
            }) => AlbumSortMethod::ArtistDesc,
            Some(TableSort {
                column: "Release Date",
                ascending: true,
            }) => AlbumSortMethod::ReleaseAsc,
            Some(TableSort {
                column: "Release Date",
                ascending: false,
            }) => AlbumSortMethod::ReleaseDesc,
            Some(TableSort {
                column: "Label",
                ascending: true,
            }) => AlbumSortMethod::LabelAsc,
            Some(TableSort {
                column: "Label",
                ascending: false,
            }) => AlbumSortMethod::LabelDesc,
            Some(TableSort {
                column: "Catalog Number",
                ascending: true,
            }) => AlbumSortMethod::CatalogAsc,
            Some(TableSort {
                column: "Catalog Number",
                ascending: false,
            }) => AlbumSortMethod::CatalogDesc,
            _ => AlbumSortMethod::ArtistAsc,
        };

        Ok(cx.list_albums(sort_method)?)
    }

    fn get_row(cx: &mut gpui::App, id: Self::Identifier) -> anyhow::Result<Option<Arc<Self>>> {
        Ok(cx.get_album_by_id(id.0 as i64, AlbumMethod::Thumbnail).ok())
    }

    fn get_column(&self, cx: &mut App, column: &'static str) -> Option<SharedString> {
        match column {
            "Title" => Some(self.title.0.clone()),
            "Artist" => cx
                .get_artist_name_by_id(self.artist_id)
                .ok()
                .map(|v| (*v).clone().into()),
            "Release Date" => self
                .release_date
                .map(|date| date.format("%x").to_string().into()),
            "Label" => self.label.as_ref().map(|v| v.0.clone()),
            "Catalog Number" => self.catalog_number.as_ref().map(|v| v.0.clone()),
            _ => None,
        }
    }

    fn get_image(&self) -> Option<Arc<RenderImage>> {
        self.thumb.as_ref().map(|thumb| thumb.0.clone())
    }
}
