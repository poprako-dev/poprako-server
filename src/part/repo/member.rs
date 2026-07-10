//! Repository traits for the membership domain.
//!
//! All member opers are transactional — the non-transactional
//! [`MemberRepo`] carries only the [`DeriveTransactional`] bound and
//! delegates entirely to [`MemberRepoTransactional`].

use poprako_transactional::advance::Advance;

use crate::part::repo::step::member::{
    Create, Delete, FindInfoByUserIdAndTeamId, GetInfoById, ListInfos,
    ListInfosByUserIdExcluded, UpdateRole, UpdateUserNickname,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional member repository.
///
/// Has no standalone opers of its own — all member steps are
/// transactional. The trait exists solely to link to
/// [`MemberRepoTransactional`] via the `C` anchor.
pub trait MemberRepo<C>:
    DeriveTransactional
    + for<'a> Execute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
where
    Self::Transactional: MemberRepoTransactional<C>,
{
}

/// Transactional member repository.
///
/// All member opers require a transaction context because they
/// are typically composed with user or team opers.
pub trait MemberRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateUserNickname<'a>, C, Error = RegularError>
    + for<'a> Advance<ListInfosByUserIdExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<FindInfoByUserIdAndTeamId<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateRole<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
    + Sized
{
}
