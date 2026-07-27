use poprako_orchestra::Oper;
use time::OffsetDateTime;

use crate::model::comic_archive::{ComicArchiveEntry, ComicArchiveSnapshot};
use crate::value::comic_archive::ComicArchiveMonth;

/// Lists archive JSON strings for selected retained month slots.
pub struct ListComicArchivePayloads<'a> {
    //
    /// The team ID.
    pub team_id: &'a str,

    /// The retained month slots to list.
    pub months: &'a [ComicArchiveMonth],
}

impl Oper for ListComicArchivePayloads<'_> {
    // List of (timestamp, payload JSON) pairs.
    type Output = Vec<(OffsetDateTime, String)>;
}

/// Retrieves an archive snapshot for a comic with excluded fields omitted.
pub struct GetComicArchiveSnapshotExcluded<'a> {
    /// The comic ID.
    pub comic_id: &'a str,
}

impl Oper for GetComicArchiveSnapshotExcluded<'_> {
    // The archive snapshot.
    type Output = ComicArchiveSnapshot;
}

/// Commits a new comic archive entry.
pub struct CommitComicArchive<'a> {
    /// The archive entry to insert.
    pub entry: &'a ComicArchiveEntry,
}

impl Oper for CommitComicArchive<'_> {
    // Unit on success.
    type Output = ();
}
