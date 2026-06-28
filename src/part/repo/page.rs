//! Repository traits for the page domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::page::{ClearImagesByChapter, DeleteByChapterId, ListByChapter};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional page repository.
pub trait PageRepo<C>: DeriveTransactional
where
    Self::Transactional: PageRepoTransactional<C>,
{
}

/// Transactional page repository.
pub trait PageRepoTransactional<C>:
    for<'a> Advance<ListByChapter<'a>, C, Error = RootError>
    // FIXME: wrong position.
    + for<'a> Advance<ClearImagesByChapter<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteByChapterId<'a>, C, Error = RootError>
{
}
