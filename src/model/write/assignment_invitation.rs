//! Domain models for chapter assignment invitations.

use crate::value::role::RoleMask;

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
