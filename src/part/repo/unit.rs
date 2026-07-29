//! Repository trait for the unit domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::unit::{
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::result::BaseError;

/// Unit repository operations over standalone runs and coordinated steps.
pub trait UnitRepo<C>:
    for<'a> Run<ListUnitInfos<'a>, Error = BaseError>
    + for<'a> Step<ListUnitInfos<'a>, C, Error = BaseError>
    + for<'a> Step<CreateUnit<'a>, C, Error = BaseError>
    + for<'a> Step<SaveUnit<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteUnit<'a>, C, Error = BaseError>
    + for<'a> Step<ListUnitIndexes<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateUnitIndexes<'a>, C, Error = BaseError>
    + for<'a> Step<CountUnits<'a>, C, Error = BaseError>
{
}
