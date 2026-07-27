use poprako_orchestra::Oper;

use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationInfo,
    AssignmentInvitationListSpec,
};

/// Creates an assignment invitation.
pub struct CreateAssignmentInvitation<'a> {
    /// The assignment invitation entry data.
    pub entry: &'a AssignmentInvitationEntry,
}

impl Oper for CreateAssignmentInvitation<'_> {
    // Internal output type for this step.
    type Output = AssignmentInvitationInfo;
}

/// Lists assignment invitation infos selected by a query specification.
pub struct ListAssignmentInvitationInfos<'a> {
    /// Query specification for filtering invitation infos.
    pub spec: &'a AssignmentInvitationListSpec,
}

impl Oper for ListAssignmentInvitationInfos<'_> {
    // Internal output type for this step.
    type Output = Vec<AssignmentInvitationInfo>;
}

/// Gets an assignment invitation that must exist.
pub enum GetAssignmentInvitationInfo<'a> {
    /// Gets by invitation identifier.
    Id {
        /// Invitation identifier.
        id: &'a str,
    },
}

impl Oper for GetAssignmentInvitationInfo<'_> {
    // Internal output type for this step.
    type Output = AssignmentInvitationInfo;
}

/// Gets an assignment invitation that must exist (with exclusive lock).
pub struct GetAssignmentInvitationInfoExcluded<'a> {
    /// Invitation code.
    pub code: &'a str,
}

impl Oper for GetAssignmentInvitationInfoExcluded<'_> {
    // Internal output type for this step.
    type Output = AssignmentInvitationInfo;
}

/// Marks an assignment invitation as used.
pub struct MarkAssignmentInvitationUsed<'a> {
    /// Invitation identifier.
    pub id: &'a str,
}

impl Oper for MarkAssignmentInvitationUsed<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Purges one expired assignment invitation when it remains pending.
pub struct PurgeExpiredAssignmentInvitation<'a> {
    /// Invitation identifier.
    pub id: &'a str,
}

impl Oper for PurgeExpiredAssignmentInvitation<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Deletes assignment invitations selected by identifier or chapter.
pub enum DeleteAssignmentInvitations<'a> {
    /// Deletes by invitation identifier.
    Id {
        /// Invitation identifier.
        id: &'a str,
    },

    /// Deletes by chapter.
    Chapter {
        /// Chapter identifier.
        chapter_id: &'a str,
    },
}

impl Oper for DeleteAssignmentInvitations<'_> {
    // Internal output type for this step.
    type Output = ();
}
