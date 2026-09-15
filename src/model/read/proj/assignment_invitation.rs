//! Domain models for chapter assignment invitations.

use time::OffsetDateTime;

use crate::value::role::RoleMask;

/// An invitation record for joining a chapter assignment.
#[cfg_attr(test, derive(Clone))]
pub struct AssignmentInvitationInfo {
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
    pub is_pending: bool,

    /// Bitmask of workflow roles the invitation would grant upon consumption.
    pub roles: RoleMask,

    /// Timestamp when the invitation was created.
    pub created_at: OffsetDateTime,
    /// Timestamp of the last modification to the invitation.
    pub updated_at: OffsetDateTime,
}
