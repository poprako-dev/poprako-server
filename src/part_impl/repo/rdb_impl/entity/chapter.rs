//! Diesel entity types for the `t_chapter` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::chapter::{ChapterEntry, ChapterInfo};
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::result::BaseError;
use crate::value::chapter::{Stage, StageMask, StagePhase};

/// Raw database row for the `t_chapter` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_chapter)]
pub struct ChapterRow {
    //
    pub f_id: String,

    pub f_comic_id: String,

    pub f_is_pinned: bool,
    pub f_index: i32,
    pub f_subtitle: String,

    pub f_page_count: i32,
    pub f_total_unit_count: i32,
    pub f_translated_unit_count: i32,
    pub f_proofread_unit_count: i32,

    pub f_uploaded_at: Option<OffsetDateTime>,
    pub f_translating_at: Option<OffsetDateTime>,
    pub f_translated_at: Option<OffsetDateTime>,
    pub f_proofreading_at: Option<OffsetDateTime>,
    pub f_proofread_at: Option<OffsetDateTime>,
    pub f_typesetting_at: Option<OffsetDateTime>,
    pub f_typeset_at: Option<OffsetDateTime>,
    pub f_reviewed_at: Option<OffsetDateTime>,
    pub f_published_at: Option<OffsetDateTime>,

    pub f_creator_id: String,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<ChapterRow> for ChapterInfo {
    type Error = BaseError;

    fn try_from(row: ChapterRow) -> Result<Self, Self::Error> {
        //
        let stages = workflow_stage_mask_from_row(&row)?;

        Ok(Self {
            id: row.f_id,
            comic_id: row.f_comic_id,
            comic: None,
            is_pinned: row.f_is_pinned,
            index: row.f_index,
            subtitle: row.f_subtitle,
            page_count: row.f_page_count,
            total_unit_count: row.f_total_unit_count,
            translated_unit_count: row.f_translated_unit_count,
            proofread_unit_count: row.f_proofread_unit_count,
            stages,
            creator_id: row.f_creator_id,
            creator: None,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

/// Insertable struct for creating a new record in the `t_chapter` table.
#[derive(Insertable)]
#[diesel(table_name = t_chapter)]
pub struct ChapterRowEntry<'a> {
    //
    pub f_id: &'a str,

    pub f_comic_id: &'a str,

    pub f_is_pinned: bool,
    pub f_index: i32,
    pub f_subtitle: &'a str,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a ChapterEntry> for ChapterRowEntry<'a> {
    fn from(chapter_entry: &'a ChapterEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &chapter_entry.id,
            f_comic_id: &chapter_entry.comic_id,
            f_is_pinned: chapter_entry.is_pinned,
            f_index: chapter_entry.index,
            f_subtitle: &chapter_entry.subtitle,
            f_creator_id: &chapter_entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}

/// Aspect struct for updating specific fields of a chapter record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_chapter)]
pub struct ChapterAspect<'a> {
    //
    pub f_is_pinned: Option<bool>,
    pub f_subtitle: Option<&'a str>,
    pub f_uploaded_at: Option<Option<OffsetDateTime>>,
    pub f_translating_at: Option<Option<OffsetDateTime>>,
    pub f_translated_at: Option<Option<OffsetDateTime>>,
    pub f_proofreading_at: Option<Option<OffsetDateTime>>,
    pub f_proofread_at: Option<Option<OffsetDateTime>>,
    pub f_typesetting_at: Option<Option<OffsetDateTime>>,
    pub f_typeset_at: Option<Option<OffsetDateTime>>,
    pub f_reviewed_at: Option<Option<OffsetDateTime>>,
    pub f_published_at: Option<Option<OffsetDateTime>>,

    pub f_page_count: Option<i32>,
    pub f_total_unit_count: Option<i32>,
    pub f_translated_unit_count: Option<i32>,
    pub f_proofread_unit_count: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> ChapterAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_is_pinned: None,
            f_subtitle: None,
            f_uploaded_at: None,
            f_translating_at: None,
            f_translated_at: None,
            f_proofreading_at: None,
            f_proofread_at: None,
            f_typesetting_at: None,
            f_typeset_at: None,
            f_reviewed_at: None,
            f_published_at: None,
            f_page_count: None,
            f_total_unit_count: None,
            f_translated_unit_count: None,
            f_proofread_unit_count: None,
            f_updated_at: updated_at,
        }
    }

    pub fn pinned(mut self, val: bool) -> Self {
        //
        self.f_is_pinned = Some(val);

        self
    }

    pub fn subtitle(mut self, val: &'a str) -> Self {
        //
        self.f_subtitle = Some(val);

        self
    }

    pub fn stages(
        mut self,
        val: StageMask,
        updated_at: OffsetDateTime,
    ) -> Self {
        //
        self.f_uploaded_at =
            Some(one_shot_timestamp(val, Stage::RawProvide, updated_at));

        (self.f_translating_at, self.f_translated_at) =
            two_step_timestamps(val, Stage::Translate, updated_at);

        (self.f_proofreading_at, self.f_proofread_at) =
            two_step_timestamps(val, Stage::Proofread, updated_at);

        (self.f_typesetting_at, self.f_typeset_at) =
            two_step_timestamps(val, Stage::TypesetRedraw, updated_at);

        self.f_reviewed_at =
            Some(one_shot_timestamp(val, Stage::Review, updated_at));

        self.f_published_at =
            Some(one_shot_timestamp(val, Stage::Publish, updated_at));

        self
    }

    pub fn page_count(mut self, val: i32) -> Self {
        //
        self.f_page_count = Some(val);

        self
    }

    pub fn total_unit_count(mut self, val: i32) -> Self {
        //
        self.f_total_unit_count = Some(val);

        self
    }

    pub fn translated_unit_count(mut self, val: i32) -> Self {
        //
        self.f_translated_unit_count = Some(val);

        self
    }

    pub fn proofread_unit_count(mut self, val: i32) -> Self {
        //
        self.f_proofread_unit_count = Some(val);

        self
    }
}

/// Convert an optional one-shot timestamp to a `StagePhase`:
/// `Some` maps to `Completed`, `None` maps to `Pending`.
fn phase_from_one_shot(timestamp: Option<OffsetDateTime>) -> StagePhase {
    match timestamp {
        //
        Some(_) => StagePhase::Completed,

        None => StagePhase::Pending,
    }
}

/// Convert optional start/completed timestamps to a `StagePhase`:
/// `(Some, Some)` -> `Completed`, `(Some, None)` -> `Active`,
/// `(None, None)` -> `Pending`.
fn phase_from_two_step(
    started_at: Option<OffsetDateTime>,
    completed_at: Option<OffsetDateTime>,
) -> StagePhase {
    match (started_at, completed_at) {
        //
        (_, Some(_)) => StagePhase::Completed,

        (Some(_), None) => StagePhase::Active,

        (None, None) => StagePhase::Pending,
    }
}

/// Resolve a one-shot stage (RawProvide, Review, Publish) to its timestamp:
/// `Some(updated_at)` when completed, `None` when pending.
fn one_shot_timestamp(
    stages: StageMask,
    stage: Stage,
    updated_at: OffsetDateTime,
) -> Option<OffsetDateTime> {
    match stages.get_phase(stage) {
        //
        StagePhase::Pending => None,

        StagePhase::Completed => Some(updated_at),

        StagePhase::Active => unreachable!("one-shot stages cannot be active"),
    }
}

/// Resolve a two-step stage (Translate, Proofread, TypesetRedraw) to its
/// start/completed timestamps. Returns `(Option<Option>, Option<Option>)` for
/// use in `ChapterAspect` fields where `Some(None)` means "clear the column"
/// and `Some(Some(ts))` means "set the column".
fn two_step_timestamps(
    stages: StageMask,
    stage: Stage,
    updated_at: OffsetDateTime,
) -> (
    Option<Option<OffsetDateTime>>,
    Option<Option<OffsetDateTime>>,
) {
    match stages.get_phase(stage) {
        //
        StagePhase::Pending => (Some(None), Some(None)),

        StagePhase::Active => (Some(Some(updated_at)), Some(None)),

        StagePhase::Completed => {
            (Some(Some(updated_at)), Some(Some(updated_at)))
        }
    }
}

/// Build a `StageMask` from a `ChapterRow` by converting each column-pair into
/// its corresponding `StagePhase`.
fn workflow_stage_mask_from_row(
    row: &ChapterRow,
) -> Result<StageMask, BaseError> {
    //
    let stages = StageMask::try_from(0u32)?
        .try_set_phase(
            Stage::RawProvide,
            phase_from_one_shot(row.f_uploaded_at),
        )?
        .try_set_phase(
            Stage::Translate,
            phase_from_two_step(row.f_translating_at, row.f_translated_at),
        )?
        .try_set_phase(
            Stage::Proofread,
            phase_from_two_step(row.f_proofreading_at, row.f_proofread_at),
        )?
        .try_set_phase(
            Stage::TypesetRedraw,
            phase_from_two_step(row.f_typesetting_at, row.f_typeset_at),
        )?
        .try_set_phase(Stage::Review, phase_from_one_shot(row.f_reviewed_at))?
        .try_set_phase(
            Stage::Publish,
            phase_from_one_shot(row.f_published_at),
        )?;

    Ok(stages)
}
