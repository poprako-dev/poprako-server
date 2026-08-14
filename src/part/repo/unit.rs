//! Repository capabilities for the Unit domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::result::BaseError;

/// Unit repository operations.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListUnitInfos<'a>,
    ),
    step(
        for<'a> ListUnitOrders<'a>,
        for<'a> ApplyUnitEdits<'a>,
    ),
)]
pub trait UnitRepo<C> {}
