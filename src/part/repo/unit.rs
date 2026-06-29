//! Repository traits for the unit domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::unit::{CountByPage, ListInfosByPage, SwapInfosByPage};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional unit repository.
pub trait UnitRepo<C>:
    DeriveTransactional + for<'a> Execute<ListInfosByPage<'a>, Error = RootError>
where
    Self::Transactional: UnitRepoTransactional<C>,
{
}

/// Transactional unit repository.
pub trait UnitRepoTransactional<C>:
    for<'a> Advance<ListInfosByPage<'a>, C, Error = RootError>
    + for<'a> Advance<SwapInfosByPage<'a>, C, Error = RootError>
    + for<'a> Advance<CountByPage<'a>, C, Error = RootError>
{
}
