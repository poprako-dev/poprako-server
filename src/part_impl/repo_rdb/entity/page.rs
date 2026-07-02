//! Diesel entity types for the `t_page` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::page::{PageForm, PageInfo};
use crate::part_impl::repo_rdb::schema::t_page;

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_page)]
pub struct PageRow {
    pub f_id: String,

    pub f_chapter_id: String,
    pub f_index: i32,

    pub f_image_key: Option<String>,
    pub f_image_uploaded: bool,
    pub f_image_version: i64,

    pub f_total_unit_count: i32,
    pub f_translated_unit_count: i32,
    pub f_proofread_unit_count: i32,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = t_page)]
pub struct PageEntry<'a> {
    pub f_id: &'a str,

    pub f_chapter_id: &'a str,
    pub f_index: i32,

    pub f_image_key: Option<&'a str>,
    pub f_image_version: i64,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = t_page)]
pub struct PageAspect<'a> {
    pub f_image_key: Option<Option<&'a str>>,
    pub f_image_uploaded: Option<bool>,
    pub f_image_version: Option<i64>,

    pub f_total_unit_count: Option<i32>,
    pub f_translated_unit_count: Option<i32>,
    pub f_proofread_unit_count: Option<i32>,

    pub f_updated_at: OffsetDateTime,
}

impl<'a> PageAspect<'a> {
    pub fn new(updated_at: OffsetDateTime) -> Self {
        Self {
            f_image_key: None,
            f_image_uploaded: None,
            f_image_version: None,
            f_total_unit_count: None,
            f_translated_unit_count: None,
            f_proofread_unit_count: None,
            f_updated_at: updated_at,
        }
    }

    pub fn image_key(mut self, val: Option<&'a str>) -> Self {
        self.f_image_key = Some(val);
        self
    }

    pub fn image_uploaded(mut self, val: bool) -> Self {
        self.f_image_uploaded = Some(val);
        self
    }

    pub fn image_version(mut self, val: i64) -> Self {
        self.f_image_version = Some(val);
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

impl From<PageRow> for PageInfo {
    fn from(row: PageRow) -> Self {
        Self {
            id: row.f_id,
            chapter_id: row.f_chapter_id,
            index: row.f_index,
            image_key: row.f_image_key,
            image_uploaded: row.f_image_uploaded,
            image_version: row.f_image_version,
            total_unit_count: row.f_total_unit_count,
            translated_unit_count: row.f_translated_unit_count,
            proofread_unit_count: row.f_proofread_unit_count,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        }
    }
}

impl<'a> From<&'a PageForm> for PageEntry<'a> {
    fn from(form: &'a PageForm) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &form.id,
            f_chapter_id: &form.chapter_id,
            f_index: form.index,
            f_image_key: form.image_key.as_deref(),
            f_image_version: form.image_version,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
