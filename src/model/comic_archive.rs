//! Snapshot and persisted-record types for immutable comic archives.

use time::OffsetDateTime;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::workset::WorksetInfo;

/// Fully locked active data used to build an immutable archive.
pub struct ComicArchiveSnapshot {
    /// The comic record being archived.
    pub comic_info: ComicInfo,
    /// The workset that contains the archived comic.
    pub workset_info: WorksetInfo,
    /// All chapters belonging to this comic, each with its own descendant snapshots.
    pub chapter_snapshots: Vec<ComicArchiveChapterSnapshot>,
}

/// Active descendants belonging to one archived chapter.
pub struct ComicArchiveChapterSnapshot {
    /// The chapter record being archived.
    pub chapter_info: ChapterInfo,
    /// Assignments linked to this chapter at the time of archiving.
    pub assignment_infos: Vec<AssignmentInfo>,
    /// All pages under this chapter, each containing its text units.
    pub page_snapshots: Vec<ComicArchivePageSnapshot>,
}

/// Active page data and its ordered text units.
pub struct ComicArchivePageSnapshot {
    /// The page record being archived.
    pub page_info: PageInfo,
    /// Ordered text units belonging to this page at the time of archiving.
    pub unit_infos: Vec<UnitInfo>,
}

/// One compressed row to persist in an archive table.
#[derive(Clone)]
pub struct ComicArchiveRecord {
    /// Unique identifier for the archive record.
    pub id: String,
    /// The team that owns the archived comic.
    pub team_id: String,
    /// Serialised JSON snapshot of the comic, its chapters, pages, and units.
    pub archived_payload: String,
    /// The user who triggered the archiving operation.
    pub archiver_id: String,
    /// When this archive record was created.
    pub created_at: OffsetDateTime,
}

/// Archive rows and the source IDs that must be deleted atomically.
pub struct ComicArchiveWrite {
    /// The archive record to insert.
    pub record: ComicArchiveRecord,
    /// The archived comic's original ID — this record will be deleted after archiving.
    pub source_comic_id: String,
    /// IDs of all chapters that were archived and should be deleted.
    pub source_chapter_ids: Vec<String>,
    /// IDs of all pages that were archived and should be deleted.
    pub source_page_ids: Vec<String>,
}
