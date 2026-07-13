//! Repository trait for the assignment invitation domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
};
use crate::result::RegularError;

/// Assignment invitation repository operations over standalone runs and coordinated steps.
pub trait AssignmentInvitationRepo<C>:
    for<'a> Run<ListAssignmentInvitationInfos<'a>, Error = RegularError>
    + for<'a> Run<GetAssignmentInvitationInfo<'a>, Error = RegularError>
    + for<'a> Step<CreateAssignmentInvitation<'a>, C, Error = RegularError>
    + for<'a> Step<GetAssignmentInvitationInfo<'a>, C, Error = RegularError>
    + for<'a> Step<
        GetAssignmentInvitationInfoExcluded<'a>,
        C,
        Error = RegularError,
    > + for<'a> Step<MarkAssignmentInvitationUsed<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteAssignmentInvitations<'a>, C, Error = RegularError>
{
}
