//! Repository trait for the unit domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::unit::{
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::result::RegularError;

/// Unit repository operations over standalone runs and coordinated steps.
pub trait UnitRepo<C>:
    for<'a> Run<ListUnitInfos<'a>, Error = RegularError>
    + for<'a> Step<ListUnitInfos<'a>, C, Error = RegularError>
    + for<'a> Step<CreateUnit<'a>, C, Error = RegularError>
    + for<'a> Step<SaveUnit<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteUnit<'a>, C, Error = RegularError>
    + for<'a> Step<ListUnitIndexes<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateUnitIndexes<'a>, C, Error = RegularError>
    + for<'a> Step<CountUnits<'a>, C, Error = RegularError>
{
}
