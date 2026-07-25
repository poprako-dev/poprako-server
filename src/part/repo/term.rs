use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, LockTerm, UpdateTerm,
};
use crate::result::BaseError;

/// Terminology-entry repository operations.
///
/// Independent reads use [`Run`]. Mutations and pessimistic reads use [`Step`]
/// with the context coordinated by the caller.
pub trait TermRepo<C>:
    for<'a> Run<GetTermInfo<'a>, Error = BaseError>
    + for<'a> Run<ListTermInfos<'a>, Error = BaseError>
    + for<'a> Step<CreateTerm<'a>, C, Error = BaseError>
    + for<'a> Step<GetTermInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<LockTerm<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateTerm<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteTerm<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteTerms<'a>, C, Error = BaseError>
{
}

impl<T, C> TermRepo<C> for T where
    T: for<'a> Run<GetTermInfo<'a>, Error = BaseError>
        + for<'a> Run<ListTermInfos<'a>, Error = BaseError>
        + for<'a> Step<CreateTerm<'a>, C, Error = BaseError>
        + for<'a> Step<GetTermInfoExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<LockTerm<'a>, C, Error = BaseError>
        + for<'a> Step<UpdateTerm<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteTerm<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteTerms<'a>, C, Error = BaseError>
{
}
