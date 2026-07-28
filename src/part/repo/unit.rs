//! Repository capabilities for the Unit domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::result::BaseError;

/// Complete Unit repository capability used by application harnesses.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListUnitInfos<'a>,
    ),
)]
pub trait UnitRepo<C>: UnitRepoTransactional<C> {}

/// Unit capabilities executed inside the caller-owned transaction.
#[drive(
    context = C,
    error = BaseError,
    step(
        for<'a> ListUnitOrders<'a>,
        for<'a> ApplyUnitEdits<'a>,
    ),
)]
pub trait UnitRepoTransactional<C> {}
