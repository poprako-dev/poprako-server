use poprako_orchestra::Oper;
use time::OffsetDateTime;

use crate::model::read::proj::comic_archive::ComicArchiveSnapshot;
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::value::comic_archive::ComicArchiveMonth;

/// Lists archive JSON strings for selected month slots.
#[derive(Oper)]
#[oper(output = Vec<(OffsetDateTime, String)>)]
pub struct ListComicArchivePayloads<'a> {
    //
    /// The team ID.
    pub team_id: &'a str,

    /// The month slots to list.
    pub months: &'a [ComicArchiveMonth],
}

/// Retrieves an archive snapshot for a comic with excluded fields omitted.
#[derive(Oper)]
#[oper(output = ComicArchiveSnapshot)]
pub struct GetComicArchiveSnapshotExcluded<'a> {
    /// The comic ID.
    pub comic_id: &'a str,
}

/// Commits a new comic archive entry.
#[derive(Oper)]
#[oper(output = ())]
pub struct CommitComicArchive<'a> {
    /// The archive entry to insert.
    pub entry: &'a ComicArchiveEntry,
}
