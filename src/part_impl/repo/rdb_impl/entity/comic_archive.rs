//! Diesel entries for immutable comic archive rows.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::comic_archive::ComicArchiveRecord;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;

/// Insertable row for `t_comic_archive`.
#[derive(Insertable)]
#[diesel(table_name = t_comic_archive)]
pub struct ComicArchiveEntry<'a> {
    pub f_id: &'a str,
    pub f_team_id: &'a str,
    pub f_archived_bytes: &'a [u8],
    pub f_archiver_id: &'a str,
    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ComicArchiveRecord> for ComicArchiveEntry<'a> {
    fn from(comic_archive_record: &'a ComicArchiveRecord) -> Self {
        Self {
            f_id: &comic_archive_record.id,
            f_team_id: &comic_archive_record.team_id,
            f_archived_bytes: &comic_archive_record.archived_bytes,
            f_archiver_id: &comic_archive_record.archiver_id,
            f_created_at: comic_archive_record.created_at,
        }
    }
}
