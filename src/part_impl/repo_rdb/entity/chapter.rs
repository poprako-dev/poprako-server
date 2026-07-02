//! Diesel entity types for the `t_chapter` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::chapter::{ChapterForm, ChapterInfo};
use crate::part_impl::repo_rdb::schema::t_chapter;
use crate::result::RegularError;
use crate::value::chapter::WorkflowStageMask;

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

    pub f_stages: i32,

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

    pub f_stages: i32,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = t_chapter)]
pub struct ChapterAspect<'a> {
    pub f_is_pinned: Option<bool>,
    pub f_subtitle: Option<&'a str>,
    pub f_stages: Option<i32>,

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
            f_stages: None,
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

    pub fn stages(mut self, val: WorkflowStageMask) -> Self {
        self.f_stages = Some(u32::from(val) as i32);
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

impl TryFrom<ChapterRow> for ChapterInfo {
    type Error = RegularError;

    fn try_from(row: ChapterRow) -> Result<Self, Self::Error> {
        let stages = WorkflowStageMask::try_from(row.f_stages as u32)?;

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
            f_stages: 0,
            f_creator_id: &form.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
