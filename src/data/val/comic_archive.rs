//! Val DTOs for the comic archive domain.
//! Request and response values for immutable comic archive operations.
/// Value returned after a comic has been archived atomically.
#[cfg_attr(test, derive(Debug))]
pub struct ArchiveComicVal {
    /// Identifier of the archived comic.
    pub archived_id: String,
}

/// Stored JSON archive payloads for one UTC month slot.
pub struct ComicArchiveExportMonthVal {
    /// UTC month slot in `YYYY-MM` format.
    pub month: String,

    /// Stored JSON archive payload strings for this month.
    pub archives: Vec<String>,
}

/// Stored JSON archive payloads grouped by UTC month slot.
pub struct ExportComicArchivesVal {
    /// Selected month slots in the requested order.
    pub months: Vec<ComicArchiveExportMonthVal>,
}
