//! Diesel entity types for the `t_workset` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::workset::{WorksetForm, WorksetInfo};
use crate::part_impl::repo::rdb_impl::schema::t_workset;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_workset` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_workset)]
pub struct WorksetRow {
    pub f_id: String,
    pub f_team_id: String,
    pub f_index: i32,

    pub f_name: String,
    pub f_description: Option<String>,

    pub f_comic_count: i32,
    pub f_comic_next_index: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_workset` table.
#[derive(Insertable)]
#[diesel(table_name = t_workset)]
pub struct WorksetEntry<'a> {
    pub f_id: &'a str,
    pub f_team_id: &'a str,
    pub f_index: i32,

    pub f_name: &'a str,
    pub f_description: Option<&'a str>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a workset record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_workset)]
pub struct WorksetAspect<'a> {
    pub f_name: Option<&'a str>,
    pub f_description: Option<Option<&'a str>>,

    pub f_comic_count: Option<i32>,
    pub f_comic_next_index: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> WorksetAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_name: None,
            f_description: None,
            f_comic_count: None,
            f_comic_next_index: None,
            f_updated_at: updated_at,
        }
    }

    pub fn name(mut self, val: &'a str) -> Self {
        self.f_name = Some(val);
        self
    }

    pub fn description(mut self, val: Option<&'a str>) -> Self {
        self.f_description = Some(val);
        self
    }

    pub fn comic_count(mut self, val: i32) -> Self {
        self.f_comic_count = Some(val);
        self
    }

    pub fn comic_next_index(mut self, val: i32) -> Self {
        self.f_comic_next_index = Some(val);
        self
    }
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<WorksetRow> for WorksetInfo {
    fn from(v: WorksetRow) -> Self {
        WorksetInfo {
            id: v.f_id,
            team_id: v.f_team_id,
            index: v.f_index,
            name: v.f_name,
            description: v.f_description,
            comic_count: v.f_comic_count,
            comic_next_index: v.f_comic_next_index,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        }
    }
}

impl<'a> From<&'a WorksetForm> for WorksetEntry<'a> {
    fn from(form: &'a WorksetForm) -> Self {
        Self {
            f_id: &form.id,
            f_team_id: &form.team_id,
            f_index: form.index,
            f_name: &form.name,
            f_description: form.description.as_deref(),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }
}
