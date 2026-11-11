//! Instr DTOs for the assignment invitation domain.
//! Data transfer objects for assignment invitation use cases.
#[cfg(not(debug_assertions))]
use veil::Redact;

use crate::value::pagination::PubListLimit;
use crate::value::role::RoleMask;

/// Input parameters for listing invitations under one chapter.
///
/// Example: `/api/v1/assignment-invitations?chapter_id=c_1&is_pending=true&offset=0&limit=20`.
#[derive(Debug)]
pub struct ListAssignmentInvitationInfosInstr {
    /// Parent chapter whose assignment invitations to list.
    pub chapter_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}

/// Input parameters for creating an assignment invitation.
#[derive(Debug)]
pub struct CreateAssignmentInvitationInstr {
    /// Identifier of the chapter to create the invitation for.
    pub chapter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,
    /// Role mask to assign to the invitee upon joining.
    pub roles: RoleMask,
}

/// Input parameters for joining an assignment through an invitation code.
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
pub struct JoinAssignmentInvitationInstr {
    /// Secret invitation code to join with.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub code: String,
}
