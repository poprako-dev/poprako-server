//! Repository traits for the unit domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::unit::{
    CountByPageId, CreateInfo, DeleteByIdInPage, ListIndexesByPageId, ListInfosByPageId, SaveInfo,
    UpdateIndexesByPageId,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional unit repository.
pub trait UnitRepo<C>:
    DeriveTransactional + for<'a> Execute<ListInfosByPageId<'a>, Error = RootError>
where
    Self::Transactional: UnitRepoTransactional<C>,
{
}

/// Transactional unit repository.
pub trait UnitRepoTransactional<C>:
    for<'a> Advance<ListInfosByPageId<'a>, C, Error = RootError>
    + for<'a> Advance<CreateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<SaveInfo<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteByIdInPage<'a>, C, Error = RootError>
    + for<'a> Advance<ListIndexesByPageId<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateIndexesByPageId<'a>, C, Error = RootError>
    + for<'a> Advance<CountByPageId<'a>, C, Error = RootError>
{
}
