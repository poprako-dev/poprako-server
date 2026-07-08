//! Repository traits for the workset domain.
//!
//! Workset opers are always transactional — listing with a lock and
//! cascade-deleting are composed with team opers.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::workset::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrComicNextIndex, ListAllInfosByTeamIdExcluded,
    ListInfosByTeamId, UpdateComicCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional workset repository.
///
/// Has no standalone opers — delegates entirely to
/// [`WorksetRepoTransactional`].
pub trait WorksetRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListInfosByTeamId<'a>, Error = RegularError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RegularError>
where
    Self::Transactional: WorksetRepoTransactional<C>,
{
}

/// Transactional workset repository.
pub trait WorksetRepoTransactional<C>:
    for<'a> Advance<ListAllInfosByTeamIdExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RegularError>
    + for<'a> Advance<Create<'a>, C, Error = RegularError>
    // NOTE: As the concurrency is expected not to be so high in production environment,
    // excluded row lock is acceptable now.
    + for<'a> Advance<IncrComicNextIndex<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateComicCount<'a>, C, Error = RegularError>
{
}
