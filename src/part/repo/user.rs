//! Repository traits for the user domain.
//!
//! Follows the two-trait pattern described in the [repo module](super):
//! [`UserRepo`] provides non-transactional opers on pool connections,
//! while [`UserRepoTransactional`] provides opers that run inside a
//! [`Drive::with_context`] transaction.
//!
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

use poprako_transactional::advance::Advance;

use crate::part::repo::step::user::{
    Create, Delete, FindInfoByQid, GetCredentialByQid, GetInfoById, GetInfoExcluded,
    MarkAvatarUploaded, ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional user repository.
///
/// Provides standalone read and write opers that each use their own
/// database connection. These are suitable for simple queries and updates
/// that do not need to be atomic with other steps.
///
/// The `C` type parameter anchors the transactional associated type — see
/// the [repo module](super) for details.
pub trait UserRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<GetCredentialByQid<'a>, Error = RegularError>
    + for<'a> Execute<FindInfoByQid<'a>, Error = RegularError>
where
    Self::Transactional: UserRepoTransactional<C>,
{
}

/// Transactional user repository.
///
/// Operations in this trait must run inside a [`Drive::with_context`]
/// block. They share the transaction's mutable context `C` and are
/// committed or rolled back atomically.
///
/// # Included opers
///
/// | Step | Purpose |
/// |------|---------|
/// | [`Create`] | Insert a new user |
/// | [`UpdateInfo`] | Update QQ ID and nickname |
/// | [`ReserveAvatar`] | Reserve an avatar upload slot |
/// | [`MarkAvatarUploaded`] | Confirm avatar upload completed |
/// | [`TouchLastActive`] | Update last-active timestamp |
/// | [`GetInfoExcluded`] | Fetch with pessimistic lock |
/// | [`Delete`] | Delete a user |
///
/// [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context
pub trait UserRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<FindInfoByQid<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RegularError>
    + for<'a> Advance<ReserveAvatar<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RegularError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
{
}
