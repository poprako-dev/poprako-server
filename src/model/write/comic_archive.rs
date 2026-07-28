//! Snapshot and persisted-record types for immutable comic archives.

use crate::model::read::proj::comic_archive::ComicArchiveRecord;

/// Archive rows and the source IDs that must be deleted atomically.
pub struct ComicArchiveEntry {
    //
    /// The archive record to insert.
    pub record: ComicArchiveRecord,
    /// The archived comic's original ID — this record will be deleted after archiving.
    pub source_comic_id: String,
    /// IDs of all chapters that were archived and should be deleted.
    pub source_chapter_ids: Vec<String>,
    /// IDs of all pages that were archived and should be deleted.
    pub source_page_ids: Vec<String>,
}
