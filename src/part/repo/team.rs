//! Repository traits for the team domain.
//!
//! Mirrors the pattern in [`UserRepo`]: a non-transactional trait for
//! standalone operations and a transactional trait for operations that
//! must participate in a [`Drive::with_context`] closure.
//!
//! [`UserRepo`]: super::user::UserRepo
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, List, MarkAvatarUploaded, ReserveAvatar,
    UpdateInfo,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional team repository.
///
/// Unlike [`UserRepo`], the team non-transactional surface includes
/// [`Create`] and [`UpdateInfo`] — these are simple single-row writes
/// that do not require transactional atomicity.
///
/// The `C` type parameter anchors the transactional associated type — see
/// the [repo module](super) for details.
pub trait TeamRepo<C>:
    DeriveTransactional
    + for<'a> Execute<Create<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + Execute<List, Error = RootError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<MarkAvatarUploaded<'a>, Error = RootError>
where
    Self::Transactional: TeamRepoTransactional<C>,
{
}

/// Transactional team repository.
///
/// Operations that must run inside a [`Drive::with_context`] closure.
///
/// [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context
pub trait TeamRepoTransactional<C>:
    for<'a> Advance<ReserveAvatar<'a>, C, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
