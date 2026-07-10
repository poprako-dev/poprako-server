//! Repository trait for immutable comic archive transactions.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::comic_archive::{Commit, LockSnapshot};
use crate::result::RegularError;

/// Transactional repository operations for one atomic comic archive.
pub trait ComicArchiveRepoTransactional<C>:
    for<'a> Advance<LockSnapshot<'a>, C, Error = RegularError>
    + for<'a> Advance<Commit<'a>, C, Error = RegularError>
{
}
