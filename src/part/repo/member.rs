//! Repository traits for the membership domain.
//!
//! All member operations are transactional — the non-transactional
//! [`MemberRepo`] carries only the [`DeriveTransactional`] bound and
//! delegates entirely to [`MemberRepoTransactional`].

use poprako_transactional::advance::Advance;

use crate::part::repo::step::member::{
    Create, Delete, ListByUserIdExcluded, TouchLastActive, UpdateUserNickname,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional member repository.
///
/// Has no standalone operations of its own — all member steps are
/// transactional. The trait exists solely to link to
/// [`MemberRepoTransactional`] via the `C` anchor.
pub trait MemberRepo<C>: DeriveTransactional
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
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
    + Sized
{
}
