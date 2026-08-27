#![allow(clippy::ref_option_ref)]

//! Diesel entity types for the `t_workset` table.
//!
//! FIXME: Diesel's generated `AsChangeset` implementation reports
//! `ref_option_ref` for the intentional tri-state `Option<Option<&T>>` field.
//! Flattening it would change the unchanged, clear, and set update semantics.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::WorksetEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset;
use crate::result::BaseError;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_workset` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_workset)]
pub struct WorksetInfoRow {
    //
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

impl TryFrom<WorksetInfoRow> for WorksetInfo {
    type Error = BaseError;

    fn try_from(v: WorksetInfoRow) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            id: v.f_id,
            team_id: v.f_team_id,
            index: usize_from_i32(v.f_index, "t_workset.f_index")?,
            name: v.f_name,
            description: v.f_description,
            comic_count: usize_from_i32(
                v.f_comic_count,
                "t_workset.f_comic_count",
            )?,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_workset` table.
#[derive(Insertable)]
#[diesel(table_name = t_workset)]
pub struct WorksetEntryRow<'a> {
    //
    pub f_id: &'a str,
    pub f_team_id: &'a str,
    pub f_index: i32,

    pub f_name: &'a str,
    pub f_description: Option<&'a str>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> TryFrom<&'a WorksetEntry> for WorksetEntryRow<'a> {
    type Error = BaseError;

    fn try_from(workset_entry: &'a WorksetEntry) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            f_id: &workset_entry.id,
            f_team_id: &workset_entry.team_id,
            f_index: i32_from_usize(workset_entry.index, "t_workset.f_index")?,
            f_name: &workset_entry.name,
            f_description: workset_entry.description.as_deref(),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        })
    }
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a workset record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_workset)]
pub struct WorksetAspectRow<'a> {
    //
    pub f_name: Option<&'a str>,
    pub f_description: Option<Option<&'a str>>,

    pub f_comic_count: Option<i32>,
    pub f_comic_next_index: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> WorksetAspectRow<'a> {
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        //
        Self {
            f_name: None,
            f_description: None,
            f_comic_count: None,
            f_comic_next_index: None,
            f_updated_at: updated_at,
        }
    }

    pub const fn name(mut self, val: &'a str) -> Self {
        //
        self.f_name = Some(val);

        self
    }

    pub const fn description(mut self, val: Option<&'a str>) -> Self {
        //
        self.f_description = Some(val);

        self
    }

    pub const fn comic_count(mut self, val: i32) -> Self {
        //
        self.f_comic_count = Some(val);

        self
    }

    pub const fn comic_next_index(mut self, val: i32) -> Self {
        //
        self.f_comic_next_index = Some(val);

        self
    }
}
