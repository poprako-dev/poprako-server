//! Repository traits for the assignment invitation domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::assignment_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional assignment invitation repository.
pub trait AssignmentInvitationRepo<C>:
    DeriveTransactional
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
where
    Self::Transactional: AssignmentInvitationRepoTransactional<C>,
{
}

/// Transactional assignment invitation repository.
pub trait AssignmentInvitationRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
{
}
