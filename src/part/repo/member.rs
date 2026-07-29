//! Repository traits for the membership domain.
//!
//! The repository object executes standalone reads through [`Run`] and
//! advances writes and locks through the context supplied to [`Step`].

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    ListMemberInfosExcluded, UpdateMember,
};
use crate::result::BaseError;

/// Member repository operations.
///
/// Independent reads use [`Run`]. Mutations, transactional reads, and
/// pessimistic locks use [`Step`] with the context coordinated by the caller.
pub trait MemberRepo<C>:
    for<'a> Run<FindMemberInfo<'a>, Error = BaseError>
    + for<'a> Run<ListMemberInfos<'a>, Error = BaseError>
    + for<'a, 'b> Run<GetMemberInfo<'a, 'b>, Error = BaseError>
    + for<'a> Step<CreateMember<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateMember<'a>, C, Error = BaseError>
    + for<'a> Step<ListMemberInfos<'a>, C, Error = BaseError>
    + for<'a> Step<FindMemberInfo<'a>, C, Error = BaseError>
    + for<'a, 'b> Step<GetMemberInfo<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<ListMemberInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteMember<'a>, C, Error = BaseError>
{
}
