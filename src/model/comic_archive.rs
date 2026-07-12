//! Snapshot and persisted-record types for immutable comic archives.

use bitcode::{Decode, Encode};
use time::OffsetDateTime;

use crate::model::{
    assignment_model, chapter_model, comic_model, page_model, unit_model,
    workset_model,
};

/// Fully locked active data used to build an immutable archive.
pub struct Snapshot {
    pub comic_info: comic_model::Info,
    pub workset_info: workset_model::Info,
    pub chapter_snapshots: Vec<ChapterSnapshot>,
}

/// Active descendants belonging to one archived chapter.
pub struct ChapterSnapshot {
    pub chapter_info: chapter_model::Info,
    pub assignment_infos: Vec<assignment_model::Info>,
    pub page_snapshots: Vec<PageSnapshot>,
}

/// Active page data and its ordered text units.
pub struct PageSnapshot {
    pub page_info: page_model::Info,
    pub unit_infos: Vec<unit_model::Info>,
}

/// One compressed row to persist in an archive table.
#[derive(Clone)]
pub struct Record {
    pub id: String,
    pub archived_bytes: Vec<u8>,
    pub archiver_id: String,
    pub created_at: OffsetDateTime,
}

/// Archive rows and the source IDs that must be deleted atomically.
pub struct Write {
    pub comic_record: Record,
    pub chapter_records: Vec<Record>,
    pub translation_records: Vec<Record>,
    pub source_comic_id: String,
    pub source_chapter_ids: Vec<String>,
    pub source_page_ids: Vec<String>,
}

/// Immutable comic payload stored in `t_archived_comic`.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedPayload {
    pub source_comic_id: String,
    pub workset: ArchivedWorksetPayload,
    pub index: i32,
    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub chapter_count: i32,
    pub chapter_next_index: i32,
    pub creator_id: String,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub chapter_archive_ids: Vec<String>,
}

/// Embedded workset snapshot belonging to an archived comic.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedWorksetPayload {
    pub id: String,
    pub team_id: String,
    pub index: i32,
    pub name: String,
    pub description: Option<String>,
    pub comic_count: i32,
    pub comic_next_index: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Immutable chapter payload stored in `t_archived_chapter`.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedChapterPayload {
    pub source_chapter_id: String,
    pub archived_comic_id: String,
    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub stages: u32,
    pub creator_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub assignments: Vec<ArchivedAssignmentPayload>,
}

/// Assignment role snapshot paired with the complete assigned-user profile.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedAssignmentPayload {
    pub source_assignment_id: String,
    pub user_id: String,
    pub roles: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub user: ArchivedUserPayload,
}

/// User profile embedded in a chapter archive.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedUserPayload {
    pub id: String,
    pub qid: String,
    pub nickname: String,
    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: u32,
    pub is_sadmin: bool,
    pub last_active_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Immutable page and unit payload stored in `t_archived_translation`.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedTranslationPayload {
    pub source_chapter_id: String,
    pub archived_chapter_id: String,
    pub pages: Vec<ArchivedPagePayload>,
}

/// Page snapshot excluding object-storage state and image metadata.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedPagePayload {
    pub source_page_id: String,
    pub index: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub units: Vec<ArchivedUnitPayload>,
}

/// Unit snapshot retained in its page order.
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct ArchivedUnitPayload {
    pub source_unit_id: String,
    pub index: i32,
    pub is_bubble: bool,
    pub is_proofread: bool,
    pub x_coord: f64,
    pub y_coord: f64,
    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,
    pub proofread_text: Option<String>,
    pub last_proofreader_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
