//! Repository trait for immutable comic archive transactions.

use poprako_orchestra::drive;

use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, DeleteComicArchives, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::result::BaseError;

/// Comic archive operations within a caller-coordinated transaction.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListComicArchivePayloads<'a>,
    ),
    step(
        for<'a> GetComicArchiveSnapshotExcluded<'a>,
        for<'a> CommitComicArchive<'a>,
        for<'a> DeleteComicArchives<'a>,
    ),
)]
pub trait ComicArchiveRepo<C> {}
