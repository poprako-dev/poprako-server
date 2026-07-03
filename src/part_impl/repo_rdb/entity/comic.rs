//! Diesel entity types for the `t_comic` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{ComicForm, ComicInfo};
use crate::part_impl::repo_rdb::schema::t_comic;

// ── Queryable / Selectable ─────────────────────────────────────────────────

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_comic)]
pub struct ComicRow {
    pub f_id: String,
    pub f_workset_id: String,
    pub f_index: i32,

    pub f_title: String,
    pub f_author: String,
    pub f_description: Option<String>,
    pub f_composed_title: String,
    pub f_is_completed: bool,

    pub f_cover_key: Option<String>,
    pub f_cover_uploaded: bool,
    pub f_cover_version: i64,

    pub f_chapter_count: i32,
    pub f_chapter_next_index: i32,

    pub f_creator_id: String,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = t_comic)]
pub struct ComicEntry<'a> {
    pub f_id: &'a str,
    pub f_workset_id: &'a str,
    pub f_index: i32,

    pub f_title: &'a str,
    pub f_author: &'a str,
    pub f_description: Option<&'a str>,
    pub f_composed_title: String,

    pub f_creator_id: &'a str,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

#[derive(AsChangeset)]
#[diesel(table_name = t_comic)]
pub struct ComicAspect<'a> {
    pub f_title: Option<&'a str>,
    pub f_author: Option<&'a str>,
    pub f_description: Option<Option<&'a str>>,
    pub f_composed_title: Option<String>,

    pub f_is_completed: Option<bool>,

    pub f_cover_key: Option<&'a str>,
    pub f_cover_uploaded: Option<bool>,
    pub f_cover_version: Option<i64>,

    pub f_chapter_count: Option<i32>,
    pub f_chapter_next_index: Option<i32>,

    pub f_last_active_at: Option<OffsetDateTime>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> ComicAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_title: None,
            f_author: None,
            f_description: None,
            f_composed_title: None,
            f_is_completed: None,
            f_cover_key: None,
            f_cover_uploaded: None,
            f_cover_version: None,
            f_chapter_count: None,
            f_chapter_next_index: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn title(mut self, val: &'a str) -> Self {
        self.f_title = Some(val);
        self
    }

    pub fn author(mut self, val: &'a str) -> Self {
        self.f_author = Some(val);
        self
    }

    pub fn description(mut self, val: Option<&'a str>) -> Self {
        self.f_description = Some(val);
        self
    }

    pub fn composed_title(mut self, val: String) -> Self {
        self.f_composed_title = Some(val);
        self
    }

    pub fn completed(mut self, val: bool) -> Self {
        self.f_is_completed = Some(val);
        self
    }

    pub fn cover_key(mut self, val: &'a str) -> Self {
        self.f_cover_key = Some(val);
        self
    }

    pub fn cover_uploaded(mut self, val: bool) -> Self {
        self.f_cover_uploaded = Some(val);
        self
    }

    pub fn cover_version(mut self, val: i64) -> Self {
        self.f_cover_version = Some(val);
        self
    }

    pub fn chapter_count(mut self, val: i32) -> Self {
        self.f_chapter_count = Some(val);
        self
    }

    pub fn chapter_next_index(mut self, val: i32) -> Self {
        self.f_chapter_next_index = Some(val);
        self
    }

    pub fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        self.f_last_active_at = Some(val);
        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<ComicRow> for ComicInfo {
    fn from(v: ComicRow) -> Self {
        ComicInfo {
            id: v.f_id,
            workset_id: v.f_workset_id,
            index: v.f_index,
            title: v.f_title,
            author: v.f_author,
            description: v.f_description,
            is_completed: v.f_is_completed,
            cover_key: v.f_cover_key,
            cover_uploaded: v.f_cover_uploaded,
            cover_version: v.f_cover_version,
            chapter_count: v.f_chapter_count,
            chapter_next_index: v.f_chapter_next_index,
            creator_id: v.f_creator_id,
            workset: None,
            team: None,
            creator: None,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}

impl<'a> From<&'a ComicForm> for ComicEntry<'a> {
    fn from(form: &'a ComicForm) -> Self {
        Self {
            f_id: &form.id,
            f_workset_id: &form.workset_id,
            f_index: form.index,
            f_title: &form.title,
            f_author: &form.author,
            f_description: form.description.as_deref(),
            f_composed_title: ComicComplex::compose_title(form.index, &form.author, &form.title),
            f_creator_id: &form.creator_id,
            f_last_active_at: OffsetDateTime::now_utc(),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }
}
