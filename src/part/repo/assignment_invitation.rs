//! Repository traits for the assignment invitation domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::assignment_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional assignment invitation repository.
pub trait AssignmentInvitationRepo<C>:
    DeriveTransactional
    + for<'a> Execute<ListInfos<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
where
    Self::Transactional: AssignmentInvitationRepoTransactional<C>,
{
}

/// Transactional assignment invitation repository.
pub trait AssignmentInvitationRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
