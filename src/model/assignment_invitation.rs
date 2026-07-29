//! Domain models for chapter assignment invitations.

use time::OffsetDateTime;

use crate::value::role::RoleMask;

/// An invitation record for joining a chapter assignment.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInvitationInfo {
    //
    /// Unique identifier for the invitation record.
    pub id: String,

    /// Foreign key to the chapter the invitation grants access to.
    pub chapter_id: String,

    /// Foreign key to the user who created the invitation.
    pub inviter_id: String,
    /// Qualified identifier of the targeted invitee user.
    pub invitee_qid: String,

    /// Unique secret token that the invitee presents to consume the invitation.
    pub code: String,

    /// Whether the invitation is still awaiting consumption.
    pub pending: bool,

    /// Bitmask of workflow roles the invitation would grant upon consumption.
    pub roles: RoleMask,

    /// Timestamp when the invitation was created.
    pub created_at: OffsetDateTime,
    /// Timestamp of the last modification to the invitation.
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert an assignment invitation row.
pub struct AssignmentInvitationEntry {
    //
    /// Unique identifier to insert for the new invitation row.
    pub id: String,

    /// Foreign key identifying the chapter to grant access to.
    pub chapter_id: String,

    /// Foreign key identifying the user extending the invitation.
    pub inviter_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,

    /// Unique secret token generated for this invitation.
    pub code: String,

    /// Bitmask of workflow roles offered by this invitation.
    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing assignment invitations.
pub struct AssignmentInvitationListSpec {
    //
    /// Foreign key scoping the listing to invitations for this chapter.
    pub chapter_id: String,
    /// Consumption status filter controlling which subset of invitations to return.
    pub kind: AssignmentInvitationListKind,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}

/// Consumption-status filtering mode for listing assignment invitations.
pub enum AssignmentInvitationListKind {
    /// Include invitations regardless of consumption status.
    All,

    /// Include only invitations that have not yet been consumed.
    Pending,

    /// Include only invitations that have already been consumed.
    Used,
}
