//! Domain models for member invitations.

use crate::value::role::RoleMask;

/// The data needed to insert a member invitation row.
pub struct MemberInvitationEntry {
    //
    /// Unique identifier for the new invitation record.
    pub id: String,

    /// The team the invitation grants access to.
    pub team_id: String,

    /// The user creating and sending the invitation.
    pub invitor_id: String,
    /// Qualified identifier of the user being invited.
    pub invitee_qid: String,

    /// Opaque token the invitee presents to accept the invitation.
    pub code: String,

    /// Role mask to assign upon acceptance of the invitation.
    pub roles: RoleMask,
}

/// Mutable fields for a member invitation.
pub struct MemberInvitationRoleRepl {
    //
    /// Identifies which invitation record to update.
    pub id: String,
    /// Updated role mask to assign upon acceptance.
    pub roles: RoleMask,
}
