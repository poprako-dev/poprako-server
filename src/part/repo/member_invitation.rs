//! Repository traits for the member invitation domain.
//!
//! Invitation opers are always transactional — fetching an invitation
//! with a lock and marking it as consumed must happen atomically during
//! registration.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::member_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional member invitation repository.
///
/// Has no standalone opers — delegates entirely to
/// [`MemberInvitationRepoTransactional`].
pub trait MemberInvitationRepo<C>:
    DeriveTransactional
    + for<'a> Execute<ListInfos<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
where
    Self::Transactional: MemberInvitationRepoTransactional<C>,
{
}

/// Transactional member invitation repository.
pub trait MemberInvitationRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
