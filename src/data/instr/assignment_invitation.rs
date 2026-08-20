//! Instr DTOs for the assignment invitation domain.

//! Data transfer objects for assignment invitation use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::value::role::RoleMask;

/// Input parameters for listing invitations under one chapter.
///
/// Example: `/api/v1/assignment-invitations?chapter_id=c_1&is_pending=true&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAssignmentInvitationInfosInstr {
    /// Parent chapter whose assignment invitations to list.
    pub chapter_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub is_pending: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for creating an assignment invitation.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationInstr {
    /// Identifier of the chapter to create the invitation for.
    pub chapter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,
    /// Role mask to assign to the invitee upon joining.
    pub roles: RoleMask,
}

/// Input parameters for joining an assignment through an invitation code.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct JoinAssignmentInvitationInstr {
    /// Secret invitation code to join with.
    pub code: String,
}
