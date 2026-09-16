//! Repository capabilities for the Unit domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitInfosByPageIds,
    ListUnitInfosInChapterOrder, ListUnitOrders, SearchChapterUnitIds,
};
use crate::part::repo::oper::unit_save::{FindUnitSave, InsertUnitSave};
use crate::result::BaseError;

/// Unit repository operations.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> ListUnitInfosByPageIds<'a>),
    step(
        for<'a> ListUnitInfosByIds<'a>,
        for<'a> ListUnitInfosInChapterOrder<'a>,
        for<'a> ListUnitOrders<'a>,
        for<'a> SearchChapterUnitIds<'a>,
        for<'a> ApplyUnitEdits<'a>,
    ),
)]
pub trait UnitRepo<C> {}

/// Durable receipts used by serializable Unit saves.
#[drive(context = C, error = BaseError,
    step(for<'a> FindUnitSave<'a>, for<'a> InsertUnitSave<'a>),
)]
pub trait UnitSaveRepo<C> {}
