//! Repository traits for the workset domain.
//!
//! Workset operations are always transactional — listing with a lock and
//! cascade-deleting are composed with team operations.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::workset::{DeleteCascade, ListByTeamIdExcluded};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional workset repository.
///
/// Has no standalone operations — delegates entirely to
/// [`WorksetRepoTransactional`].
pub trait WorksetRepo<C>: DeriveTransactional
where
    Self::Transactional: WorksetRepoTransactional<C>,
{
}

/// Transactional workset repository.
pub trait WorksetRepoTransactional<C>:
    for<'a> Advance<ListByTeamIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteCascade<'a>, C, Error = RootError>
{
}
