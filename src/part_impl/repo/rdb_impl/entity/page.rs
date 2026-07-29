//! Diesel entity types for the `t_page` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::page::{PageEntry, PageInfo};
use crate::part_impl::repo::rdb_impl::schema::t_page;
use crate::result::BaseError;
use crate::value::image::{ImageExt, ImageHash};

/// Raw database row for the `t_page` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_page)]
pub struct PageRow {
    //
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_index: i32,

    pub f_image_key: Option<String>,
    pub f_image_uploaded: bool,
    #[diesel(deserialize_as = i64)]
    pub f_image_version: u32,
    pub f_image_hash: Vec<u8>,
    pub f_image_extension: String,

    pub f_total_unit_count: i32,
    pub f_translated_unit_count: i32,
    pub f_proofread_unit_count: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Insertable struct for creating a new record in the `t_page` table.
#[derive(Insertable)]
#[diesel(table_name = t_page)]
pub struct PageRowEntry<'a> {
    //
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,
    pub f_index: i32,

    pub f_image_key: Option<&'a str>,
    pub f_image_version: i64,
    pub f_image_hash: Vec<u8>,
    pub f_image_extension: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

/// Aspect struct for updating specific fields of a page record identified by id.
#[derive(AsChangeset)]
#[diesel(table_name = t_page)]
pub struct PageAspect<'a> {
    //
    pub f_index: Option<i32>,
    pub f_image_key: Option<Option<&'a str>>,
    pub f_image_uploaded: Option<bool>,
    pub f_image_version: Option<i64>,
    pub f_image_hash: Option<&'a [u8]>,
    pub f_image_extension: Option<&'a str>,

    pub f_total_unit_count: Option<i32>,
    pub f_translated_unit_count: Option<i32>,
    pub f_proofread_unit_count: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> PageAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_index: None,
            f_image_key: None,
            f_image_uploaded: None,
            f_image_version: None,
            f_image_hash: None,
            f_image_extension: None,
            f_total_unit_count: None,
            f_translated_unit_count: None,
            f_proofread_unit_count: None,
            f_updated_at: updated_at,
        }
    }

    pub fn index(mut self, val: i32) -> Self {
        //
        self.f_index = Some(val);

        self
    }

    pub fn image_key(mut self, val: Option<&'a str>) -> Self {
        //
        self.f_image_key = Some(val);

        self
    }

    pub fn image_uploaded(mut self, val: bool) -> Self {
        //
        self.f_image_uploaded = Some(val);

        self
    }

    pub fn image_version(mut self, val: u32) -> Self {
        //
        self.f_image_version = Some(i64::from(val));

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

impl TryFrom<PageRow> for PageInfo {
    type Error = BaseError;

    fn try_from(row: PageRow) -> Result<Self, Self::Error> {
        //
        let image_hash_bytes: [u8; 32] =
            row.f_image_hash.try_into().map_err(|_| {
                BaseError::Unrecoverable {
                    message: "[PageRow] f_image_hash must contain 32 bytes"
                        .into(),
                }
            })?;

        let image_extension = ImageExt::parse(&row.f_image_extension)
            .ok_or_else(|| BaseError::Unrecoverable {
                message: "[PageRow] f_image_extension must be supported".into(),
            })?;

        Ok(Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            index: row.f_index,
            image_key: row.f_image_key,
            image_uploaded: row.f_image_uploaded,
            image_version: row.f_image_version,
            image_hash: ImageHash::new(image_hash_bytes),
            image_ext: image_extension,
            total_unit_count: row.f_total_unit_count,
            translated_unit_count: row.f_translated_unit_count,
            proofread_unit_count: row.f_proofread_unit_count,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        })
    }
}

impl<'a> TryFrom<&'a PageEntry> for PageRowEntry<'a> {
    type Error = BaseError;

    fn try_from(entry: &'a PageEntry) -> Result<Self, Self::Error> {
        //
        let now = OffsetDateTime::now_utc();

        Ok(Self {
            f_id: &entry.id,
            f_chapter_id: &entry.chapter_id,
            f_index: entry.index,
            f_image_key: entry.image_key.as_deref(),
            f_image_version: i64::from(entry.image_version),
            f_image_hash: entry.image_hash.bytes().to_vec(),
            f_image_extension: entry.image_ext.suffix(),
            f_created_at: now,
            f_updated_at: now,
        })
    }
}
