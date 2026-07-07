//! Repository traits for the team domain.
//!
//! Mirrors the pattern in [`UserRepo`]: a non-transactional trait for
//! standalone opers and a transactional trait for opers that
//! must participate in a [`Drive::with_context`] block.
//!
//! [`UserRepo`]: super::user::UserRepo
//! [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context

use poprako_transactional::advance::Advance;

use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex, ListInfos,
    MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
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
    + for<'a> Execute<Create<'a>, Error = RegularError>
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RegularError>
    + for<'a> Execute<MarkAvatarUploaded<'a>, Error = RegularError>
where
    Self::Transactional: TeamRepoTransactional<C>,
{
}

/// Transactional team repository.
///
/// Operations that must run inside a [`Drive::with_context`] block.
///
/// [`Drive::with_context`]: poprako_transactional::drive::Drive::with_context
pub trait TeamRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<ReserveAvatar<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
    + for<'a> Advance<IncrementWorksetNextIndex<'a>, C, Error = RegularError>
{
}
