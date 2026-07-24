use poprako_orchestra::Oper;
use time::OffsetDateTime;

use crate::model::comic_archive::{ComicArchiveEntry, ComicArchiveSnapshot};
use crate::value::comic_archive::ComicArchiveMonth;

/// Lists archive JSON strings for selected retained month slots.
pub struct ListComicArchivePayloads<'a> {
    //
    pub team_id: &'a str,

    pub months: &'a [ComicArchiveMonth],
}

impl Oper for ListComicArchivePayloads<'_> {
    type Output = Vec<(OffsetDateTime, String)>;
}

pub struct GetComicArchiveSnapshotExcluded<'a> {
    pub comic_id: &'a str,
}

impl Oper for GetComicArchiveSnapshotExcluded<'_> {
    type Output = ComicArchiveSnapshot;
}

pub struct CommitComicArchive<'a> {
    pub entry: &'a ComicArchiveEntry,
}

impl Oper for CommitComicArchive<'_> {
    type Output = ();
}
