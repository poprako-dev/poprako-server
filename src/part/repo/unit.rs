//! Repository capabilities for the Unit domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitInfosByIds, ListUnitInfosByPageIds,
    ListUnitInfosInChapterOrder, ListUnitOrders, SearchChapterUnitIds,
};
use crate::result::BaseError;

/// Unit repository operations.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListUnitInfos<'a>,
        for<'a> ListUnitInfosByPageIds<'a>,
    ),
    step(
        for<'a> ListUnitInfosByIds<'a>,
        for<'a> ListUnitInfosInChapterOrder<'a>,
        for<'a> ListUnitOrders<'a>,
        for<'a> SearchChapterUnitIds<'a>,
        for<'a> ApplyUnitEdits<'a>,
    ),
)]
pub trait UnitRepo<C> {}
