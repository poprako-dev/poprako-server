//! Repository capabilities for the Unit domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::result::BaseError;

/// Complete Unit repository capability used by application harnesses.
pub trait UnitRepo<C>:
    for<'a> Run<ListUnitInfos<'a>, Error = BaseError> + UnitRepoTransactional<C>
{
}

impl<T, C> UnitRepo<C> for T where
    T: for<'a> Run<ListUnitInfos<'a>, Error = BaseError>
        + UnitRepoTransactional<C>
{
}

/// Unit capabilities executed inside the caller-owned transaction.
pub trait UnitRepoTransactional<C>:
    for<'a> Step<ListUnitOrders<'a>, C, Error = BaseError>
    + for<'a> Step<ApplyUnitEdits<'a>, C, Error = BaseError>
{
}

impl<T, C> UnitRepoTransactional<C> for T where
    T: for<'a> Step<ListUnitOrders<'a>, C, Error = BaseError>
        + for<'a> Step<ApplyUnitEdits<'a>, C, Error = BaseError>
{
}
