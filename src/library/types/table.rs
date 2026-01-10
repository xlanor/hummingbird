use std::sync::Arc;

use gpui::{App, SharedString};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

use super::{Album, Track};
use crate::{
    library::db::{AlbumMethod, AlbumSortMethod, LibraryAccess, TrackSortMethod},
    ui::components::table::table_data::{Column, TableData, TableSort},
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum AlbumColumn {
    Title,
    Artist,
    Date,
    Label,
    CatalogNumber,
}

impl Column for AlbumColumn {
    fn get_column_name(&self) -> &'static str {
        match self {
            AlbumColumn::Title => "Title",
            AlbumColumn::Artist => "Artist",
            AlbumColumn::Date => "Date",
            AlbumColumn::Label => "Label",
            AlbumColumn::CatalogNumber => "Catalog Number",
        }
    }
}

impl TableData<AlbumColumn> for Album {
    type Identifier = (u32, String);

    fn get_table_name() -> &'static str {
        "Albums"
    }

    fn get_rows(
        cx: &mut gpui::App,
        sort: Option<TableSort<AlbumColumn>>,
    ) -> anyhow::Result<Vec<Self::Identifier>> {
        let sort_method = match sort {
            Some(TableSort {
                column: AlbumColumn::Title,
                ascending: true,
            }) => AlbumSortMethod::TitleAsc,
            Some(TableSort {
                column: AlbumColumn::Title,
                ascending: false,
            }) => AlbumSortMethod::TitleDesc,
            Some(TableSort {
                column: AlbumColumn::Artist,
                ascending: true,
            }) => AlbumSortMethod::ArtistAsc,
            Some(TableSort {
                column: AlbumColumn::Artist,
                ascending: false,
            }) => AlbumSortMethod::ArtistDesc,
            Some(TableSort {
                column: AlbumColumn::Date,
                ascending: true,
            }) => AlbumSortMethod::ReleaseAsc,
            Some(TableSort {
                column: AlbumColumn::Date,
                ascending: false,
            }) => AlbumSortMethod::ReleaseDesc,
            Some(TableSort {
                column: AlbumColumn::Label,
                ascending: true,
            }) => AlbumSortMethod::LabelAsc,
            Some(TableSort {
                column: AlbumColumn::Label,
                ascending: false,
            }) => AlbumSortMethod::LabelDesc,
            Some(TableSort {
                column: AlbumColumn::CatalogNumber,
                ascending: true,
            }) => AlbumSortMethod::CatalogAsc,
            Some(TableSort {
                column: AlbumColumn::CatalogNumber,
                ascending: false,
            }) => AlbumSortMethod::CatalogDesc,
            _ => AlbumSortMethod::ArtistAsc,
        };

        Ok(cx.list_albums(sort_method)?)
    }

    fn get_row(cx: &mut gpui::App, id: Self::Identifier) -> anyhow::Result<Option<Arc<Self>>> {
        Ok(cx.get_album_by_id(id.0 as i64, AlbumMethod::Thumbnail).ok())
    }

    fn get_column(&self, cx: &mut App, column: AlbumColumn) -> Option<SharedString> {
        match column {
            AlbumColumn::Title => Some(self.title.0.clone()),
            AlbumColumn::Artist => cx
                .get_artist_name_by_id(self.artist_id)
                .ok()
                .map(|v| (*v).clone().into()),
            AlbumColumn::Date => self
                .release_date
                .map(|date| date.format("%x").to_string().into()),
            AlbumColumn::Label => self.label.as_ref().map(|v| v.0.clone()),
            AlbumColumn::CatalogNumber => self.catalog_number.as_ref().map(|v| v.0.clone()),
        }
    }

    fn get_image_path(&self) -> Option<SharedString> {
        Some(format!("!db://album/{}/thumb", self.id).into())
    }

    fn has_images() -> bool {
        true
    }

    fn column_monospace(_column: AlbumColumn) -> bool {
        false
    }

    fn get_element_id(&self) -> impl Into<gpui::ElementId> {
        ("album", self.id as u32)
    }

    fn get_table_id(&self) -> Self::Identifier {
        (self.id as u32, self.title.0.clone().into())
    }

    fn default_columns() -> IndexMap<AlbumColumn, f32, FxBuildHasher> {
        let s = FxBuildHasher;
        let mut columns: IndexMap<AlbumColumn, f32, FxBuildHasher> = IndexMap::with_hasher(s);
        columns.insert(AlbumColumn::Title, 300.0);
        columns.insert(AlbumColumn::Artist, 200.0);
        columns.insert(AlbumColumn::Date, 100.0);
        columns.insert(AlbumColumn::Label, 150.0);
        columns.insert(AlbumColumn::CatalogNumber, 200.0);
        columns
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum TrackColumn {
    TrackNumber,
    Title,
    Album,
    Artist,
    Length,
}

impl Column for TrackColumn {
    fn get_column_name(&self) -> &'static str {
        match self {
            TrackColumn::TrackNumber => "#",
            TrackColumn::Title => "Title",
            TrackColumn::Album => "Album",
            TrackColumn::Artist => "Artist",
            TrackColumn::Length => "Length",
        }
    }
}

impl TableData<TrackColumn> for Track {
    type Identifier = (i64, String, Option<i64>, String);

    fn get_table_name() -> &'static str {
        "Tracks"
    }

    fn get_rows(
        cx: &mut gpui::App,
        sort: Option<TableSort<TrackColumn>>,
    ) -> anyhow::Result<Vec<Self::Identifier>> {
        let sort_method = match sort {
            Some(TableSort {
                column: TrackColumn::Title,
                ascending: true,
            }) => TrackSortMethod::TitleAsc,
            Some(TableSort {
                column: TrackColumn::Title,
                ascending: false,
            }) => TrackSortMethod::TitleDesc,
            Some(TableSort {
                column: TrackColumn::Artist,
                ascending: true,
            }) => TrackSortMethod::ArtistAsc,
            Some(TableSort {
                column: TrackColumn::Artist,
                ascending: false,
            }) => TrackSortMethod::ArtistDesc,
            Some(TableSort {
                column: TrackColumn::Album,
                ascending: true,
            }) => TrackSortMethod::AlbumAsc,
            Some(TableSort {
                column: TrackColumn::Album,
                ascending: false,
            }) => TrackSortMethod::AlbumDesc,
            Some(TableSort {
                column: TrackColumn::Length,
                ascending: true,
            }) => TrackSortMethod::DurationAsc,
            Some(TableSort {
                column: TrackColumn::Length,
                ascending: false,
            }) => TrackSortMethod::DurationDesc,
            Some(TableSort {
                column: TrackColumn::TrackNumber,
                ascending: true,
            }) => TrackSortMethod::TrackNumberAsc,
            Some(TableSort {
                column: TrackColumn::TrackNumber,
                ascending: false,
            }) => TrackSortMethod::TrackNumberDesc,
            _ => TrackSortMethod::ArtistAsc,
        };

        Ok(cx.list_tracks(sort_method)?)
    }

    fn get_row(cx: &mut gpui::App, id: Self::Identifier) -> anyhow::Result<Option<Arc<Self>>> {
        Ok(cx.get_track_by_id(id.0).ok())
    }

    fn get_column(&self, cx: &mut App, column: TrackColumn) -> Option<SharedString> {
        match column {
            TrackColumn::TrackNumber => match (self.disc_number, self.track_number) {
                (Some(disc), Some(track)) => Some(format!("{}-{}", disc, track).into()),
                (None, Some(track)) => Some(track.to_string().into()),
                _ => None,
            },
            TrackColumn::Title => Some(self.title.0.clone()),
            TrackColumn::Album => {
                if let Some(album_id) = self.album_id {
                    cx.get_album_by_id(album_id, AlbumMethod::Thumbnail)
                        .ok()
                        .map(|v| v.title.0.clone())
                } else {
                    None
                }
            }
            TrackColumn::Artist => {
                if let Some(artist) = &self.artist_names {
                    Some(artist.0.clone())
                } else if let Some(album_id) = self.album_id {
                    cx.get_album_by_id(album_id, AlbumMethod::Thumbnail)
                        .ok()
                        .and_then(|album| {
                            cx.get_artist_name_by_id(album.artist_id)
                                .ok()
                                .map(|v| (*v).clone().into())
                        })
                } else {
                    None
                }
            }
            TrackColumn::Length => {
                let minutes = self.duration / 60;
                let seconds = self.duration % 60;
                Some(format!("{:02}:{:02}", minutes, seconds).into())
            }
        }
    }

    fn get_image_path(&self) -> Option<SharedString> {
        None
    }

    fn has_images() -> bool {
        false
    }

    fn column_monospace(column: TrackColumn) -> bool {
        matches!(column, TrackColumn::Length | TrackColumn::TrackNumber)
    }

    fn get_element_id(&self) -> impl Into<gpui::ElementId> {
        ("track", self.id as u32)
    }

    fn get_table_id(&self) -> Self::Identifier {
        (
            self.id,
            self.title.0.clone().into(),
            self.album_id,
            self.location.to_string_lossy().to_string(),
        )
    }

    fn default_columns() -> IndexMap<TrackColumn, f32, FxBuildHasher> {
        let s = FxBuildHasher;
        let mut columns: IndexMap<TrackColumn, f32, FxBuildHasher> = IndexMap::with_hasher(s);
        columns.insert(TrackColumn::TrackNumber, 75.0);
        columns.insert(TrackColumn::Title, 350.0);
        columns.insert(TrackColumn::Album, 250.0);
        columns.insert(TrackColumn::Artist, 225.0);
        columns.insert(TrackColumn::Length, 100.0);
        columns
    }
}
