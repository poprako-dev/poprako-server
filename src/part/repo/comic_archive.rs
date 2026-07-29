//! Repository trait for immutable comic archive transactions.

use poprako_orchestra::Step;

use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::result::BaseError;

/// Comic archive operations within a caller-coordinated transaction.
pub trait ComicArchiveRepo<C>:
    for<'a> Step<GetComicArchiveSnapshotExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<CommitComicArchive<'a>, C, Error = BaseError>
{
}
