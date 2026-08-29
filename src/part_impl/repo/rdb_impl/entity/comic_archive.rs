//! Diesel entries for immutable comic archive rows.

use diesel::Insertable;
use time::OffsetDateTime;

use crate::model::read::proj::comic_archive::ComicArchiveRecord;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;

/// Insertable row for `t_comic_archive`.
#[derive(Insertable)]
#[diesel(table_name = t_comic_archive)]
pub struct ComicArchiveEntryRow<'a> {
    pub f_id: &'a str,
    pub f_team_id: &'a str,
    pub f_source_comic_id: &'a str,
    pub f_archived_payload: &'a str,
    pub f_archiver_id: &'a str,
    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ComicArchiveRecord> for ComicArchiveEntryRow<'a> {
    fn from(comic_archive_record: &'a ComicArchiveRecord) -> Self {
        //
        Self {
            f_id: &comic_archive_record.id,
            f_team_id: &comic_archive_record.team_id,
            f_source_comic_id: &comic_archive_record.source_comic_id,
            f_archived_payload: &comic_archive_record.archived_payload,
            f_archiver_id: &comic_archive_record.archiver_id,
            f_created_at: comic_archive_record.created_at,
        }
    }
}
