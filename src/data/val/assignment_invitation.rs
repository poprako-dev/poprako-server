//! Val DTOs for the assignment-invitation domain.

//! Data transfer objects for assignment invitation use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating an assignment invitation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAssignmentInvitationVal {
    //
    /// Unique identifier of the newly created invitation.
    pub id: String,
    /// Secret invitation code for the invitee to use.
    pub code: String,
}
