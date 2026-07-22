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
    pub comic_info: ComicInfo,
    pub workset_info: WorksetInfo,
    pub chapter_snapshots: Vec<ComicArchiveChapterSnapshot>,
}

/// Active descendants belonging to one archived chapter.
pub struct ComicArchiveChapterSnapshot {
    pub chapter_info: ChapterInfo,
    pub assignment_infos: Vec<AssignmentInfo>,
    pub page_snapshots: Vec<ComicArchivePageSnapshot>,
}

/// Active page data and its ordered text units.
pub struct ComicArchivePageSnapshot {
    pub page_info: PageInfo,
    pub unit_infos: Vec<UnitInfo>,
}

/// One compressed row to persist in an archive table.
#[derive(Clone)]
pub struct ComicArchiveRecord {
    pub id: String,
    pub team_id: String,
    pub archived_payload: String,
    pub archiver_id: String,
    pub created_at: OffsetDateTime,
}

/// Archive rows and the source IDs that must be deleted atomically.
pub struct ComicArchiveWrite {
    pub record: ComicArchiveRecord,
    pub source_comic_id: String,
    pub source_chapter_ids: Vec<String>,
    pub source_page_ids: Vec<String>,
}
