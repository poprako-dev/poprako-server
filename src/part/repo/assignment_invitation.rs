//! Repository trait for the assignment invitation domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
    PurgeExpiredAssignmentInvitation,
};
use crate::result::BaseError;

/// Assignment invitation repository operations over standalone runs and coordinated steps.
pub trait AssignmentInvitationRepo<C>: for<'a> Run<ListAssignmentInvitationInfos<'a>, Error = BaseError>
    + for<'a> Run<GetAssignmentInvitationInfo<'a>, Error = BaseError>
    + for<'a> Step<CreateAssignmentInvitation<'a>, C, Error = BaseError>
    + for<'a> Step<GetAssignmentInvitationInfo<'a>, C, Error = BaseError>
    + for<'a> Step<GetAssignmentInvitationInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<MarkAssignmentInvitationUsed<'a>, C, Error = BaseError>
    + for<'a> Step<PurgeExpiredAssignmentInvitation<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteAssignmentInvitations<'a>, C, Error = BaseError>
{
}

impl<T, C> AssignmentInvitationRepo<C> for T
where
    T: for<'a> Run<ListAssignmentInvitationInfos<'a>, Error = BaseError>
       + for<'a> Run<GetAssignmentInvitationInfo<'a>, Error = BaseError>
       + for<'a> Step<CreateAssignmentInvitation<'a>, C, Error = BaseError>
       + for<'a> Step<GetAssignmentInvitationInfo<'a>, C, Error = BaseError>
       + for<'a> Step<GetAssignmentInvitationInfoExcluded<'a>, C, Error = BaseError>
       + for<'a> Step<MarkAssignmentInvitationUsed<'a>, C, Error = BaseError>
       + for<'a> Step<PurgeExpiredAssignmentInvitation<'a>, C, Error = BaseError>
       + for<'a> Step<DeleteAssignmentInvitations<'a>, C, Error = BaseError>,
{
}
