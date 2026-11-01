//! Val DTOs for the comic archive domain.

//! Request and response values for immutable comic archive operations.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Value returned after a comic has been archived atomically.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct ArchiveComicVal {
    /// Identifier of the archived comic.
    pub archived_id: String,
}

/// Stored JSON archive payloads for one UTC month slot.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicArchiveExportMonthVal {
    /// UTC month slot in `YYYY-MM` format.
    pub month: String,

    /// Stored JSON archive payload strings for this month.
    pub archives: Vec<String>,
}

/// Stored JSON archive payloads grouped by UTC month slot.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportComicArchivesVal {
    /// Selected month slots in the requested order.
    pub months: Vec<ComicArchiveExportMonthVal>,
}
