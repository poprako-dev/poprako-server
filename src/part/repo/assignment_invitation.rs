//! Repository trait for the assignment invitation domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
    PurgeExpiredAssignmentInvitation,
};
use crate::result::BaseError;

/// Assignment invitation repository operations over standalone runs and coordinated steps.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListAssignmentInvitationInfos<'a>,
        for<'a> GetAssignmentInvitationInfo<'a>,
        for<'a> PurgeExpiredAssignmentInvitation<'a>,
    ),
    step(
        for<'a> CreateAssignmentInvitation<'a>,
        for<'a> GetAssignmentInvitationInfo<'a>,
        for<'a> GetAssignmentInvitationInfoExcluded<'a>,
        for<'a> MarkAssignmentInvitationUsed<'a>,
        for<'a> PurgeExpiredAssignmentInvitation<'a>,
        for<'a> DeleteAssignmentInvitations<'a>,
    ),
)]
pub trait AssignmentInvitationRepo<C> {}
