//! Diesel items for immutable comic archive records.
use diesel::Insertable;
use time::OffsetDateTime;

use crate::model::read::proj::comic_archive::ComicArchiveInfo;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive;

/// Insertable record for `t_comic_archive`.
#[derive(Insertable)]
#[diesel(table_name = t_comic_archive)]
pub struct ComicArchiveItem<'a> {
    pub f_id: &'a str,

    pub f_team_id: &'a str,
    pub f_source_comic_id: &'a str,
    pub f_archived_payload: &'a str,
    pub f_archiver_id: &'a str,

    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a ComicArchiveInfo> for ComicArchiveItem<'a> {
    fn from(record: &'a ComicArchiveInfo) -> Self {
        Self {
            f_id: &record.id,
            f_team_id: &record.team_id,
            f_source_comic_id: &record.source_comic_id,
            f_archived_payload: &record.archived_payload,
            f_archiver_id: &record.archiver_id,
            f_created_at: record.created_at,
        }
    }
}
