use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::result::BaseError;

/// Terminology-base repository operations.
///
/// Independent reads use [`Run`]. Mutations and pessimistic reads use [`Step`]
/// with the context coordinated by the caller.
pub trait TermbaseRepo<C>:
    for<'a> Run<GetTermbaseInfo<'a>, Error = BaseError>
    + for<'a> Run<ListTermbaseInfos<'a>, Error = BaseError>
    + for<'a> Step<CreateTermbase<'a>, C, Error = BaseError>
    + for<'a> Step<GetTermbaseInfo<'a>, C, Error = BaseError>
    + for<'a> Step<GetTermbaseInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<ListTermbaseInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateTermbase<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateTermbaseTermCount<'a>, C, Error = BaseError>
    + for<'a> Step<TouchTermbase<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteTermbase<'a>, C, Error = BaseError>
{
}
