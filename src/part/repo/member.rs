//! Repository traits for the membership domain.
//!
//! The repository object executes standalone reads through [`poprako_orchestra::Run`] and
//! advances writes and locks through the context supplied to [`poprako_orchestra::Step`].

use poprako_orchestra::drive;

use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    ListMemberInfosExcluded, UpdateMember,
};
use crate::result::BaseError;

/// Member repository operations.
///
/// Independent reads use [`poprako_orchestra::Run`]. Mutations, transactional reads, and
/// pessimistic locks use [`poprako_orchestra::Step`] with the context coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> FindMemberInfo<'a>,
        for<'a> ListMemberInfos<'a>,
        for<'a, 'b> GetMemberInfo<'a, 'b>,
    ),
    step(
        for<'a> CreateMember<'a>,
        for<'a> UpdateMember<'a>,
        for<'a> ListMemberInfos<'a>,
        for<'a> FindMemberInfo<'a>,
        for<'a, 'b> GetMemberInfo<'a, 'b>,
        for<'a> ListMemberInfosExcluded<'a>,
        for<'a> DeleteMember<'a>,
    ),
)]
pub trait MemberRepo<C> {}
