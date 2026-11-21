//! Val DTOs for the assignment-invitation domain.
//! Data transfer objects for assignment invitation use cases.
/// Return value from creating an assignment invitation.
pub struct CreateAssignmentInvitationVal {
    /// Unique identifier of the newly created invitation.
    pub id: String,
    /// Secret invitation code for the invitee to use.
    pub code: String,
}
