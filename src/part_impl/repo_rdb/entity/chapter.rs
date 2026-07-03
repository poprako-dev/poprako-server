//! Diesel entity types for the `t_chapter` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::chapter::{ChapterForm, ChapterInfo};
use crate::part_impl::repo_rdb::schema::t_chapter;
use crate::result::RegularError;
use crate::value::chapter::{StagePhase, WorkflowStage, WorkflowStageMask};

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_chapter)]
pub struct ChapterRow {
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

#[derive(Insertable)]
#[diesel(table_name = t_chapter)]
pub struct ChapterEntry<'a> {
    pub f_id: &'a str,

    pub f_comic_id: &'a str,

    pub f_is_pinned: bool,
    pub f_index: i32,
    pub f_subtitle: &'a str,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = t_chapter)]
pub struct ChapterAspect<'a> {
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
        self.f_is_pinned = Some(val);
        self
    }

    pub fn subtitle(mut self, val: &'a str) -> Self {
        self.f_subtitle = Some(val);
        self
    }

    pub fn stages(mut self, val: WorkflowStageMask, updated_at: OffsetDateTime) -> Self {
        self.f_uploaded_at = Some(one_shot_timestamp(
            val,
            WorkflowStage::RawProvide,
            updated_at,
        ));

        (self.f_translating_at, self.f_translated_at) =
            two_step_timestamps(val, WorkflowStage::Translate, updated_at);

        (self.f_proofreading_at, self.f_proofread_at) =
            two_step_timestamps(val, WorkflowStage::Proofread, updated_at);

        (self.f_typesetting_at, self.f_typeset_at) =
            two_step_timestamps(val, WorkflowStage::TypesetRedraw, updated_at);

        self.f_reviewed_at = Some(one_shot_timestamp(val, WorkflowStage::Review, updated_at));

        self.f_published_at = Some(one_shot_timestamp(val, WorkflowStage::Publish, updated_at));

        self
    }

    pub fn page_count(mut self, val: i32) -> Self {
        self.f_page_count = Some(val);
        self
    }

    pub fn total_unit_count(mut self, val: i32) -> Self {
        self.f_total_unit_count = Some(val);
        self
    }

    pub fn translated_unit_count(mut self, val: i32) -> Self {
        self.f_translated_unit_count = Some(val);
        self
    }

    pub fn proofread_unit_count(mut self, val: i32) -> Self {
        self.f_proofread_unit_count = Some(val);
        self
    }
}

fn one_shot_timestamp(
    stages: WorkflowStageMask,
    stage: WorkflowStage,
    updated_at: OffsetDateTime,
) -> Option<OffsetDateTime> {
    match stages.get_phase(stage) {
        StagePhase::Pending => None,
        StagePhase::Completed => Some(updated_at),
        StagePhase::Active => unreachable!("one-shot stages cannot be active"),
    }
}

fn two_step_timestamps(
    stages: WorkflowStageMask,
    stage: WorkflowStage,
    updated_at: OffsetDateTime,
) -> (
    Option<Option<OffsetDateTime>>,
    Option<Option<OffsetDateTime>>,
) {
    match stages.get_phase(stage) {
        StagePhase::Pending => (Some(None), Some(None)),
        StagePhase::Active => (Some(Some(updated_at)), Some(None)),
        StagePhase::Completed => (Some(Some(updated_at)), Some(Some(updated_at))),
    }
}

fn phase_from_one_shot(timestamp: Option<OffsetDateTime>) -> StagePhase {
    match timestamp {
        Some(_) => StagePhase::Completed,
        None => StagePhase::Pending,
    }
}

fn phase_from_two_step(
    started_at: Option<OffsetDateTime>,
    completed_at: Option<OffsetDateTime>,
) -> StagePhase {
    match (started_at, completed_at) {
        (_, Some(_)) => StagePhase::Completed,
        (Some(_), None) => StagePhase::Active,
        (None, None) => StagePhase::Pending,
    }
}

fn workflow_stage_mask_from_row(row: &ChapterRow) -> Result<WorkflowStageMask, RegularError> {
    let stages = WorkflowStageMask::try_from(0u32)?
        .try_set_phase(
            WorkflowStage::RawProvide,
            phase_from_one_shot(row.f_uploaded_at),
        )?
        .try_set_phase(
            WorkflowStage::Translate,
            phase_from_two_step(row.f_translating_at, row.f_translated_at),
        )?
        .try_set_phase(
            WorkflowStage::Proofread,
            phase_from_two_step(row.f_proofreading_at, row.f_proofread_at),
        )?
        .try_set_phase(
            WorkflowStage::TypesetRedraw,
            phase_from_two_step(row.f_typesetting_at, row.f_typeset_at),
        )?
        .try_set_phase(
            WorkflowStage::Review,
            phase_from_one_shot(row.f_reviewed_at),
        )?
        .try_set_phase(
            WorkflowStage::Publish,
            phase_from_one_shot(row.f_published_at),
        )?;
    Ok(stages)
}

impl TryFrom<ChapterRow> for ChapterInfo {
    type Error = RegularError;

    fn try_from(row: ChapterRow) -> Result<Self, Self::Error> {
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

impl<'a> From<&'a ChapterForm> for ChapterEntry<'a> {
    fn from(form: &'a ChapterForm) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &form.id,
            f_comic_id: &form.comic_id,
            f_is_pinned: form.is_pinned,
            f_index: form.index,
            f_subtitle: &form.subtitle,
            f_creator_id: &form.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    // derives_workflow_mask_from_timestamps(ChapterInfo::try_from)(positive): timestamp columns derive legal workflow phases.

    use super::*;

    fn row() -> ChapterRow {
        let time = OffsetDateTime::now_utc();

        ChapterRow {
            f_id: "chapter-1".into(),
            f_comic_id: "comic-1".into(),
            f_is_pinned: true,
            f_index: 0,
            f_subtitle: "Chapter".into(),
            f_page_count: 0,
            f_total_unit_count: 0,
            f_translated_unit_count: 0,
            f_proofread_unit_count: 0,
            f_uploaded_at: Some(time),
            f_translating_at: Some(time),
            f_translated_at: Some(time),
            f_proofreading_at: Some(time),
            f_proofread_at: None,
            f_typesetting_at: None,
            f_typeset_at: None,
            f_reviewed_at: Some(time),
            f_published_at: None,
            f_creator_id: "user-1".into(),
            f_created_at: time,
            f_updated_at: time,
        }
    }

    #[test]
    fn derives_workflow_mask_from_timestamps() {
        let chapter_info = ChapterInfo::try_from(row()).ok().unwrap();

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::RawProvide),
            StagePhase::Completed
        );

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::Translate),
            StagePhase::Completed
        );

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::Proofread),
            StagePhase::Active
        );

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::TypesetRedraw),
            StagePhase::Pending
        );

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::Review),
            StagePhase::Completed
        );

        assert_eq!(
            chapter_info.stages.get_phase(WorkflowStage::Publish),
            StagePhase::Pending
        );
    }
}
