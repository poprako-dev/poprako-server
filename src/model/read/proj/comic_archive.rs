//! Snapshot and persisted-record types for immutable comic archives.

use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::workset::WorksetInfo;

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
    /// Immutable workflow records retained in language-neutral form.
    pub workflow_record_infos: Vec<ChapterWorkflowRecordInfo>,
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
    /// Original comic identifier retained for archive lookup and cleanup.
    pub source_comic_id: String,
    /// Serialised JSON snapshot of the comic, its chapters, pages, and units.
    pub archived_payload: String,
    /// The user who triggered the archiving operation.
    pub archiver_id: String,

    /// When this archive record was created.
    pub created_at: OffsetDateTime,
}
