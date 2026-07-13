//! Diesel entries for immutable comic archive rows.

use crate::model::comic_archive::ComicArchiveRecord;
use diesel::prelude::*;
use time::OffsetDateTime;

use crate::part_impl::repo::rdb_impl::schema::{
    t_archived_chapter, t_archived_comic, t_archived_translation,
};

/// Insertable row for `t_archived_comic`.
#[derive(Insertable)]
#[diesel(table_name = t_archived_comic)]
pub struct ArchivedComicEntry<'a> {
    pub f_id: &'a str,
    pub f_archived_bytes: &'a [u8],
    pub f_archiver_id: &'a str,
    pub f_created_at: OffsetDateTime,
}

/// Insertable row for `t_archived_chapter`.
#[derive(Insertable)]
#[diesel(table_name = t_archived_chapter)]
pub struct ArchivedChapterEntry<'a> {
    pub f_id: &'a str,
    pub f_archived_bytes: &'a [u8],
    pub f_archiver_id: &'a str,
    pub f_created_at: OffsetDateTime,
}

/// Insertable row for `t_archived_translation`.
#[derive(Insertable)]
#[diesel(table_name = t_archived_translation)]
pub struct ArchivedTranslationEntry<'a> {
    pub f_id: &'a str,
    pub f_archived_bytes: &'a [u8],
    pub f_archiver_id: &'a str,
    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ComicArchiveRecord> for ArchivedComicEntry<'a> {
    fn from(comic_archive_record: &'a ComicArchiveRecord) -> Self {
        Self {
            f_id: &comic_archive_record.id,
            f_archived_bytes: &comic_archive_record.archived_bytes,
            f_archiver_id: &comic_archive_record.archiver_id,
            f_created_at: comic_archive_record.created_at,
        }
    }
}

impl<'a> From<&'a ComicArchiveRecord> for ArchivedChapterEntry<'a> {
    fn from(comic_archive_record: &'a ComicArchiveRecord) -> Self {
        Self {
            f_id: &comic_archive_record.id,
            f_archived_bytes: &comic_archive_record.archived_bytes,
            f_archiver_id: &comic_archive_record.archiver_id,
            f_created_at: comic_archive_record.created_at,
        }
    }
}

impl<'a> From<&'a ComicArchiveRecord> for ArchivedTranslationEntry<'a> {
    fn from(comic_archive_record: &'a ComicArchiveRecord) -> Self {
        Self {
            f_id: &comic_archive_record.id,
            f_archived_bytes: &comic_archive_record.archived_bytes,
            f_archiver_id: &comic_archive_record.archiver_id,
            f_created_at: comic_archive_record.created_at,
        }
    }
}
