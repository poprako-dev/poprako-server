//! Domain models for chapter assignment invitations.

/// Filtering and pagination parameters for listing assignment invitations.
pub struct AssignmentInvitationListSpec {
    //
    /// Foreign key scoping the listing to invitations for this chapter.
    pub chapter_id: String,
    /// Optional pending-state filter.
    pub is_pending: Option<bool>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
