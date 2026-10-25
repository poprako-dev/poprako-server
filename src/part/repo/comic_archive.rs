//! Repository trait for immutable comic archive transactions.

use poprako_orchestra::drive;

use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::result::BaseError;

/// Comic archive operations within a caller-coordinated transaction.
#[drive(
    context = C,
    error = BaseError,
    step(
        for<'a> ListComicArchivePayloads<'a>,
        for<'a> GetComicArchiveSnapshotExcluded<'a>,
        for<'a> CommitComicArchive<'a>,
    ),
)]
pub trait ComicArchiveRepo<C> {}
