#![allow(clippy::ref_option_ref)]

//! Diesel entity types for the `t_page` table.
//!
//! FIXME: Diesel's generated `AsChangeset` implementation reports
//! `ref_option_ref` for the intentional tri-state `Option<Option<&T>>` field.
//! Flattening it would change the unchanged, clear, and set update semantics.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_page;
use crate::result::BaseError;

/// Raw database row for the `t_page` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_page)]
pub struct PageInfoRow {
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_index: i32,

    pub f_total_unit_count: i32,
    pub f_translated_unit_count: i32,
    pub f_proofread_unit_count: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<PageInfoRow> for PageInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(row: PageInfoRow) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            index: usize_from_i32(row.f_index, "t_page.f_index")?,
            total_unit_count: usize_from_i32(
                row.f_total_unit_count,
                "t_page.f_total_unit_count",
            )?,
            translated_unit_count: usize_from_i32(
                row.f_translated_unit_count,
                "t_page.f_translated_unit_count",
            )?,
            proofread_unit_count: usize_from_i32(
                row.f_proofread_unit_count,
                "t_page.f_proofread_unit_count",
            )?,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

/// Insertable struct for creating a new record in the `t_page` table.
#[derive(Insertable)]
#[diesel(table_name = t_page)]
pub struct PageEntryRow<'a> {
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,
    pub f_index: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> TryFrom<&'a PageEntry> for PageEntryRow<'a> {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(entry: &'a PageEntry) -> Result<Self, Self::Error> {
        //
        let now = OffsetDateTime::now_utc();

        Ok(Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_index: i32_from_usize(entry.index, "t_page.f_index")?,
            f_created_at: now,
            f_updated_at: now,
        })
    }
}

/// Aspect struct for updating specific fields of a page record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_page)]
pub struct PageAspectRow {
    pub f_index: Option<i32>,
    pub f_total_unit_count: Option<i32>,
    pub f_translated_unit_count: Option<i32>,
    pub f_proofread_unit_count: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl PageAspectRow {
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        //
        Self {
            f_index: None,
            f_total_unit_count: None,
            f_translated_unit_count: None,
            f_proofread_unit_count: None,
            f_updated_at: updated_at,
        }
    }

    pub const fn index(mut self, val: i32) -> Self {
        //
        self.f_index = Some(val);

        self
    }

    pub const fn total_unit_count(mut self, val: i32) -> Self {
        //
        self.f_total_unit_count = Some(val);

        self
    }

    pub const fn translated_unit_count(mut self, val: i32) -> Self {
        //
        self.f_translated_unit_count = Some(val);

        self
    }

    pub const fn proofread_unit_count(mut self, val: i32) -> Self {
        //
        self.f_proofread_unit_count = Some(val);

        self
    }
}
