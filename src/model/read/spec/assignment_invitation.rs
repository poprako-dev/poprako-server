//! Domain models for chapter assignment invitations.

use crate::value::assignment_invitation::AssignmentInvitationStatus;

/// Filtering and pagination parameters for listing assignment invitations.
pub struct AssignmentInvitationListSpec {
    //
    /// Foreign key scoping the listing to invitations for this chapter.
    pub chapter_id: String,
    /// Consumption status filter controlling which subset of invitations to return.
    pub status: AssignmentInvitationStatus,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
