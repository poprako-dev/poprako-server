//! Repository traits for the member invitation domain.
//!
//! Invitation operations are always transactional — fetching an invitation
//! with a lock and marking it as consumed must happen atomically during
//! registration.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::member_invitation::{GetInfoByCodeExcluded, MarkPendingAsUsed};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional member invitation repository.
///
/// Has no standalone operations — delegates entirely to
/// [`MemberInvitationRepoTransactional`].
pub trait MemberInvitationRepo<C>: DeriveTransactional
where
    Self::Transactional: MemberInvitationRepoTransactional<C>,
{
}

/// Transactional member invitation repository.
pub trait MemberInvitationRepoTransactional<C>:
    for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RootError>
{
}
