//! Repository traits for the membership domain.
//!
//! All member operations are transactional — the non-transactional
//! [`MemberRepo`] carries only the [`DeriveTransactional`] bound and
//! delegates entirely to [`MemberRepoTransactional`].

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::member::{
    Create, Delete, FindByUserTeamId, GetInfoExcluded, ListByUserIdExcluded, ListInfos,
    TouchLastActive, UpdateRole, UpdateUserNickname,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional member repository.
///
/// Has no standalone operations of its own — all member steps are
/// transactional. The trait exists solely to link to
/// [`MemberRepoTransactional`] via the `C` anchor.
pub trait MemberRepo<C>:
    DeriveTransactional
    + for<'a> Execute<FindByUserTeamId<'a>, Error = RootError>
    + for<'a> Execute<ListInfos<'a>, Error = RootError>
where
    Self::Transactional: MemberRepoTransactional<C>,
{
}

/// Transactional member repository.
///
/// All member operations require a transaction context because they
/// are typically composed with user or team operations.
pub trait MemberRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateUserNickname<'a>, C, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RootError>
    + for<'a> Advance<ListByUserIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<FindByUserTeamId<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateRole<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
    + Sized
{
}
