//! Repository trait for immutable comic archive transactions.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::comic_archive::{CommitComicArchive, GetComicArchiveSnapshotExcluded, ListComicArchivePayloads};
use crate::result::BaseError;

/// Comic archive operations within a caller-coordinated transaction.
pub trait ComicArchiveRepo<C>:
    for<'a> Run<ListComicArchivePayloads<'a>, Error = BaseError>
    + for<'a> Step<GetComicArchiveSnapshotExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CommitComicArchive<'a>, C, Error = BaseError>
{
}

impl<T, C> ComicArchiveRepo<C> for T where
    T: for<'a> Run<ListComicArchivePayloads<'a>, Error = BaseError>
        + for<'a> Step<GetComicArchiveSnapshotExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<CommitComicArchive<'a>, C, Error = BaseError>
{
}
