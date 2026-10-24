#![allow(
    clippy::ref_option_ref,
    reason = "Diesel AsChangeset derives borrow nullable fields without changing their three-state update semantics"
)]

//! Diesel entity types for the `t_comic` table.
//!
//! FIXME: Diesel's generated `AsChangeset` implementation reports
//! `ref_option_ref` for the intentional tri-state `Option<Option<&T>>` field.
//! Flattening it would change the unchanged, clear, and set update semantics.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::complex::comic as comic_complex;
use crate::model::read::proj::comic::ComicInfo;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
use crate::part_impl::repo::rdb_impl::schema::t_comic;
use crate::result::{BaseError, BaseRest};
use crate::shape::write::comic::ComicEntryShape;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database record for the `t_comic` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_comic)]
pub struct ComicInfoRecord {
    pub f_id: String,

    pub f_workset_id: String,
    pub f_index: i32,

    pub f_title: String,
    pub f_author: String,
    pub f_description: Option<String>,
    pub f_composed_title: String,

    pub f_chapter_count: i32,
    pub f_chapter_next_index: i32,

    pub f_creator_id: String,

    pub f_last_active_at: OffsetDateTime,
    pub f_archived_at: Option<OffsetDateTime>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<ComicInfoRecord> for ComicInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(v: ComicInfoRecord) -> BaseRest<Self> {
        //
        accept!(Self {
            id: v.f_id,
            workset_id: v.f_workset_id,
            index: usize_from_i32(v.f_index, "t_comic.f_index")?,
            title: v.f_title,
            author: v.f_author,
            description: v.f_description,
            chapter_count: usize_from_i32(
                v.f_chapter_count,
                "t_comic.f_chapter_count",
            )?,
            creator_id: v.f_creator_id,
            workset: None,
            team: None,
            creator: None,
            last_active_at: v.f_last_active_at,
            archived_at: v.f_archived_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_comic` table.
#[derive(Insertable)]
#[diesel(table_name = t_comic)]
pub struct ComicItem<'a> {
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

impl<'a> TryFrom<&'a ComicEntryShape<'_>> for ComicItem<'a> {
    type Error = BaseError;

    fn try_from(entry: &'a ComicEntryShape<'_>) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            f_id: &entry.id,
            f_workset_id: entry.workset_id,
            f_index: i32_from_usize(entry.index, "t_comic.f_index")?,
            f_title: entry.trimmed_title(),
            f_author: entry.trimmed_author(),
            f_description: entry.trimmed_description(),
            f_composed_title: comic_complex::compose_title(
                entry.index,
                entry.trimmed_author(),
                entry.trimmed_title(),
            ),
            f_creator_id: entry.creator_id,
            f_last_active_at: OffsetDateTime::now_utc(),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        })
    }
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Pack struct for updating specific fields of a comic record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_comic)]
pub struct ComicPack<'a> {
    pub f_title: Option<&'a str>,
    pub f_author: Option<&'a str>,
    pub f_description: Option<Option<&'a str>>,
    pub f_composed_title: Option<String>,

    pub f_chapter_count: Option<i32>,
    pub f_chapter_next_index: Option<i32>,

    pub f_last_active_at: Option<OffsetDateTime>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> ComicPack<'a> {
    #[must_use]
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_title: None,
            f_author: None,
            f_description: None,
            f_composed_title: None,
            f_chapter_count: None,
            f_chapter_next_index: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    #[must_use]
    pub const fn title(mut self, val: &'a str) -> Self {
        //
        self.f_title = Some(val);

        self
    }

    #[must_use]
    pub const fn author(mut self, val: &'a str) -> Self {
        //
        self.f_author = Some(val);

        self
    }

    #[must_use]
    pub const fn description(mut self, val: Option<&'a str>) -> Self {
        //
        self.f_description = Some(val);

        self
    }

    #[must_use]
    pub fn composed_title(mut self, val: String) -> Self {
        //
        self.f_composed_title = Some(val);

        self
    }

    #[must_use]
    pub const fn chapter_count(mut self, val: i32) -> Self {
        //
        self.f_chapter_count = Some(val);

        self
    }

    #[must_use]
    pub const fn chapter_next_index(mut self, val: i32) -> Self {
        //
        self.f_chapter_next_index = Some(val);

        self
    }

    #[must_use]
    pub const fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_last_active_at = Some(val);

        self
    }
}
