//! Repository traits for the workset domain.
//!
//! Workset operations are always transactional — listing with a lock and
//! cascade-deleting are composed with team operations.

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::workset::{
    Create, DeleteCascade, GetInfoById, IncrementComicNextIndex, ListByTeamId,
    ListByTeamIdExcluded, UpdateComicCount, UpdateInfo,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional workset repository.
///
/// Has no standalone operations — delegates entirely to
/// [`WorksetRepoTransactional`].
pub trait WorksetRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<ListByTeamId<'a>, Error = RootError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RootError>
where
    Self::Transactional: WorksetRepoTransactional<C>,
{
}

/// Transactional workset repository.
pub trait WorksetRepoTransactional<C>:
    for<'a> Advance<ListByTeamIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteCascade<'a>, C, Error = RootError>
    + for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<IncrementComicNextIndex<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateComicCount<'a>, C, Error = RootError>
{
}
