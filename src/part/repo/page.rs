//! Repository traits for the page domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::page::ListInfosByChapter;
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
    for<'a> Advance<ListInfosByChapter<'a>, C, Error = RootError>
{
}
