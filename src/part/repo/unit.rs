//! Repository capabilities for the Unit domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitInfosByIds, ListUnitOrders,
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
        for<'a> ListUnitInfosByIds<'a>,
        for<'a> ListUnitOrders<'a>,
        for<'a> ApplyUnitEdits<'a>,
    ),
)]
pub trait UnitRepo<C> {}
