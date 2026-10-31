use poprako_orchestra::Oper;
use time::OffsetDateTime;

use crate::model::read::proj::comic_archive::{
    ComicArchivePayload, ComicArchiveSnapshot,
};
use crate::model::write::comic_archive::ComicArchiveEntry;
use crate::value::comic_archive::ComicArchiveMonth;

/// Loads the next byte-bounded prefix of immutable archive JSON strings.
#[derive(Oper)]
#[oper(output = Vec<ComicArchivePayload>)]
pub struct ListComicArchivePayloads<'a> {
    /// The team ID.
    pub team_id: &'a str,

    /// The month slots to list.
    pub months: &'a [ComicArchiveMonth],

    /// Exclusive timestamp and identity position in the selected month slots.
    pub after: Option<(OffsetDateTime, &'a str)>,
    /// Maximum combined stored payload bytes; an oversized row is an error.
    pub payload_budget_bytes: u64,
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
