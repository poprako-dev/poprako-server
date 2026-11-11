//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::comic_archive::ExportComicArchivesInstr;
use crate::data::val::comic_archive::{
    ArchiveComicVal, ComicArchiveExportMonthVal, ExportComicArchivesVal,
};

/// Query parameters for exporting selected archive month slots.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ExportComicArchivesQuery {
    #[serde(default)]
    /// UTC month slots in `YYYY-MM` format to filter archives by.
    pub month: Vec<String>,
}

impl From<ExportComicArchivesQuery> for ExportComicArchivesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ExportComicArchivesQuery) -> Self {
        Self { month: value.month }
    }
}

/// Value returned after a comic has been archived atomically.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ArchiveComicPayload {
    /// Identifier of the archived comic.
    pub archived_id: String,
}

impl From<ArchiveComicVal> for ArchiveComicPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ArchiveComicVal) -> Self {
        Self {
            archived_id: value.archived_id,
        }
    }
}

impl Wire for ArchiveComicVal {
    // Serializable wire payload produced by this conversion.
    type Target = ArchiveComicPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Stored JSON archive payloads for one UTC month slot.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicArchiveExportMonthPayload {
    /// UTC month slot in `YYYY-MM` format.
    pub month: String,

    /// Stored JSON archive payload strings for this month.
    pub archives: Vec<String>,
}

impl From<ComicArchiveExportMonthVal> for ComicArchiveExportMonthPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ComicArchiveExportMonthVal) -> Self {
        Self {
            month: value.month,
            archives: value.archives,
        }
    }
}

impl Wire for ComicArchiveExportMonthVal {
    // Serializable wire payload produced by this conversion.
    type Target = ComicArchiveExportMonthPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Stored JSON archive payloads grouped by UTC month slot.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportComicArchivesPayload {
    /// Selected month slots in the requested order.
    pub months: Vec<ComicArchiveExportMonthPayload>,
}

impl From<ExportComicArchivesVal> for ExportComicArchivesPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ExportComicArchivesVal) -> Self {
        Self {
            months: value
                .months
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for ExportComicArchivesVal {
    // Serializable wire payload produced by this conversion.
    type Target = ExportComicArchivesPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
