#![allow(clippy::ref_option_ref)]

//! Diesel entity types for the `t_comic` table.
//!
//! FIXME: Diesel's generated `AsChangeset` implementation reports
//! `ref_option_ref` for the intentional tri-state `Option<Option<&T>>` field.
//! Flattening it would change the unchanged, clear, and set update semantics.

use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::comic::ComicEntry;
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32,
};
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
    pub f_cover_uploaded: Option<bool>,
    pub f_cover_version: Option<i64>,
    pub f_cover_hash: Option<Vec<u8>>,
    pub f_cover_extension: Option<String>,

    pub f_chapter_count: i32,
    pub f_chapter_next_index: i32,

    pub f_creator_id: String,

    pub f_last_active_at: OffsetDateTime,
    pub f_archived_at: Option<OffsetDateTime>,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<ComicInfoRow> for ComicInfo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    fn try_from(v: ComicInfoRow) -> BaseRest<Self> {
        //
        let (
            cover_key,
            is_cover_uploaded,
            cover_version,
            cover_hash,
            cover_ext,
        ) = match (
            v.f_cover_key,
            v.f_cover_uploaded,
            v.f_cover_version,
            v.f_cover_hash,
            v.f_cover_extension,
        ) {
            //
            (None, None, None, None, None) => (None, None, None, None, None),

            (
                Some(cover_key),
                Some(is_cover_uploaded),
                Some(cover_version),
                Some(cover_hash),
                Some(cover_ext),
            ) => {
                //
                let cover_version = u32::try_from(cover_version).map_err(|_| {
                        //
                        BaseError::Unrecoverable {
                            message: "[ComicInfoRow] f_cover_version must be non-negative".into(),
                        }
                    })?;

                let cover_hash = cover_hash.try_into().map_err(|_| {
                    //
                    BaseError::Unrecoverable {
                        message:
                            "[ComicInfoRow] f_cover_hash must contain 32 bytes"
                                .into(),
                    }
                })?;

                let cover_ext =
                    ImageExt::parse(&cover_ext).ok_or_else(|| {
                        //
                        BaseError::Unrecoverable {
                        message:
                            "[ComicInfoRow] f_cover_extension must be supported"
                                .into(),
                    }
                    })?;

                (
                    Some(cover_key),
                    Some(is_cover_uploaded),
                    Some(cover_version),
                    Some(ImageHash::new(cover_hash)),
                    Some(cover_ext),
                )
            }

            _ => {
                //
                return Err(BaseError::Unrecoverable {
                        message: "[ComicInfoRow] cover fields must be all null or all present".into(),
                    });
            }
        };

        accept(Self {
            id: v.f_id,
            workset_id: v.f_workset_id,
            index: usize_from_i32(v.f_index, "t_comic.f_index")?,
            title: v.f_title,
            author: v.f_author,
            description: v.f_description,
            cover_key,
            is_cover_uploaded,
            cover_version,
            cover_hash,
            cover_ext,
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

impl<'a> TryFrom<&'a ComicEntry> for ComicEntryRow<'a> {
    type Error = BaseError;

    fn try_from(comic_entry: &'a ComicEntry) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            f_id: &comic_entry.id,
            f_workset_id: &comic_entry.workset_id,
            f_index: i32_from_usize(comic_entry.index, "t_comic.f_index")?,
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
        })
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
    pub const fn new(updated_at: OffsetDateTime) -> Self {
        //
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

    pub const fn title(mut self, val: &'a str) -> Self {
        //
        self.f_title = Some(val);

        self
    }

    pub const fn author(mut self, val: &'a str) -> Self {
        //
        self.f_author = Some(val);

        self
    }

    pub const fn description(mut self, val: Option<&'a str>) -> Self {
        //
        self.f_description = Some(val);

        self
    }

    pub fn composed_title(mut self, val: String) -> Self {
        //
        self.f_composed_title = Some(val);

        self
    }

    pub const fn cover_key(mut self, val: &'a str) -> Self {
        //
        self.f_cover_key = Some(val);

        self
    }

    pub const fn cover_uploaded(mut self, val: bool) -> Self {
        //
        self.f_cover_uploaded = Some(val);

        self
    }

    pub fn cover_version(mut self, val: u32) -> Self {
        //
        self.f_cover_version = Some(i64::from(val));

        self
    }

    pub const fn cover_hash(mut self, val: &'a ImageHash) -> Self {
        //
        self.f_cover_hash = Some(val.as_bytes());

        self
    }

    pub const fn cover_ext(mut self, val: ImageExt) -> Self {
        //
        self.f_cover_extension = Some(val.suffix());

        self
    }

    pub const fn chapter_count(mut self, val: i32) -> Self {
        //
        self.f_chapter_count = Some(val);

        self
    }

    pub const fn chapter_next_index(mut self, val: i32) -> Self {
        //
        self.f_chapter_next_index = Some(val);

        self
    }

    pub const fn last_active_at(mut self, val: OffsetDateTime) -> Self {
        //
        self.f_last_active_at = Some(val);

        self
    }
}
