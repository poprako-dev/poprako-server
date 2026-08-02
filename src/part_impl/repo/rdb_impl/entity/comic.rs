//! Diesel entity types for the `t_comic` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::comic::ComicEntry;
use crate::part_impl::repo::rdb_impl::schema::t_comic;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::image::{ImageExt, ImageHash};

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_comic` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_comic)]
pub struct ComicInfoRow {
    //
    pub f_id: String,
    pub f_workset_id: String,
    pub f_index: i32,

    pub f_title: String,
    pub f_author: String,
    pub f_description: Option<String>,
    pub f_composed_title: String,

    pub f_cover_key: Option<String>,
    pub f_cover_uploaded: bool,
    #[diesel(deserialize_as = i64)]
    pub f_cover_version: u32,
    pub f_cover_hash: Vec<u8>,
    pub f_cover_extension: String,

    pub f_chapter_count: i32,
    pub f_chapter_next_index: i32,

    pub f_creator_id: String,

    pub f_last_active_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<ComicInfoRow> for ComicInfo {
    type Error = BaseError;

    fn try_from(v: ComicInfoRow) -> BaseRest<Self> {
        //
        let (cover_hash_bytes, cover_ext) = (
            v.f_cover_hash.try_into().map_err(|_| {
                BaseError::Unrecoverable {
                    message:
                        "[ComicInfoRow] f_cover_hash must contain 32 bytes"
                            .into(),
                }
            })?,
            ImageExt::parse(&v.f_cover_extension).ok_or_else(|| {
                BaseError::Unrecoverable {
                    message:
                        "[ComicInfoRow] f_cover_extension must be supported"
                            .into(),
                }
            })?,
        );

        accept(ComicInfo {
            id: v.f_id,
            workset_id: v.f_workset_id,
            index: v.f_index,
            title: v.f_title,
            author: v.f_author,
            description: v.f_description,
            cover_key: v.f_cover_key,
            is_cover_uploaded: v.f_cover_uploaded,
            cover_version: v.f_cover_version,
            cover_hash: ImageHash::new(cover_hash_bytes),
            cover_ext,
            chapter_count: v.f_chapter_count,
            creator_id: v.f_creator_id,
            workset: None,
            team: None,
            creator: None,
            last_active_at: v.f_last_active_at,
            created_at: v.f_created_at,
            updated_at: v.f_updated_at,
        })
    }
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_comic` table.
#[derive(Insertable)]
#[diesel(table_name = t_comic)]
pub struct ComicEntryRow<'a> {
    //
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

impl<'a> From<&'a ComicEntry> for ComicEntryRow<'a> {
    fn from(comic_entry: &'a ComicEntry) -> Self {
        Self {
            f_id: &comic_entry.id,
            f_workset_id: &comic_entry.workset_id,
            f_index: comic_entry.index,
            f_title: &comic_entry.title,
            f_author: &comic_entry.author,
            f_description: comic_entry.description.as_deref(),
            f_composed_title: ComicComplex::compose_title(
                comic_entry.index,
                &comic_entry.author,
                &comic_entry.title,
            ),
            f_creator_id: &comic_entry.creator_id,
            f_last_active_at: OffsetDateTime::now_utc(),
            f_created_at: OffsetDateTime::now_utc(),
            f_updated_at: OffsetDateTime::now_utc(),
        }
    }
}

// ── Changeset (AsChangeset) ────────────────────────────────────────────────

/// Aspect struct for updating specific fields of a comic record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_comic)]
pub struct ComicAspectRow<'a> {
    //
    pub f_title: Option<&'a str>,
    pub f_author: Option<&'a str>,
    pub f_description: Option<Option<&'a str>>,
    pub f_composed_title: Option<String>,

    pub f_cover_key: Option<&'a str>,
    pub f_cover_uploaded: Option<bool>,
    pub f_cover_version: Option<i64>,
    pub f_cover_hash: Option<&'a [u8]>,
    pub f_cover_extension: Option<&'a str>,

    pub f_chapter_count: Option<i32>,
    pub f_chapter_next_index: Option<i32>,

    pub f_last_active_at: Option<OffsetDateTime>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> ComicAspectRow<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_title: None,
            f_author: None,
            f_description: None,
            f_composed_title: None,
            f_cover_key: None,
            f_cover_uploaded: None,
            f_cover_version: None,
            f_cover_hash: None,
            f_cover_extension: None,
            f_chapter_count: None,
            f_chapter_next_index: None,
            f_last_active_at: None,
            f_updated_at: updated_at,
        }
    }

    pub fn title(mut self, val: &'a str) -> Self {
        //
        self.f_title = Some(val);

        self
    }

    pub fn author(mut self, val: &'a str) -> Self {
        //
        self.f_author = Some(val);

        self
    }

    pub fn description(mut self, val: Option<&'a str>) -> Self {
        //
        self.f_description = Some(val);

        self
    }

    pub fn composed_title(mut self, val: String) -> Self {
        //
        self.f_composed_title = Some(val);

        self
    }

    pub fn cover_key(mut self, val: &'a str) -> Self {
        //
        self.f_cover_key = Some(val);

        self
    }

    pub fn cover_uploaded(mut self, val: bool) -> Self {
        //
        self.f_cover_uploaded = Some(val);

        self
    }

    pub fn cover_version(mut self, val: u32) -> Self {
        //
        self.f_cover_version = Some(i64::from(val));

        self
    }

    pub fn cover_hash(mut self, val: &'a ImageHash) -> Self {
        //
        self.f_cover_hash = Some(val.as_bytes());

        self
    }

    pub fn cover_ext(mut self, val: ImageExt) -> Self {
        //
        self.f_cover_extension = Some(val.suffix());

        self
    }

    pub fn chapter_count(mut self, val: i32) -> Self {
        //
        self.f_chapter_count = Some(val);

        self
    }

    pub fn chapter_next_index(mut self, val: i32) -> Self {
        //
        self.f_chapter_next_index = Some(val);

        self
    }

    pub fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_last_active_at = Some(val);

        self
    }
}
