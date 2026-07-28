//! Domain models for member invitations.

use crate::model::user::UserInfo;
use crate::value::member_invitation::{
    MemberInvitationInclOpt, MemberInvitationStatus,
};
use crate::value::role::RoleMask;

/// An invitation record for joining a team.
///
/// Carries an opaque invitation code, the inviter and invitee identifiers,
/// a pending flag indicating whether the invitation has been consumed,
/// and the [`RoleMask`] that will be assigned upon acceptance.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInvitationInfo {
    //
    /// Unique identifier for the invitation record.
    pub id: String,

    /// The team the invitation grants access to.
    pub team_id: String,

    /// The resolved inviter user record, populated when the include option is set.
    pub invitor: Option<UserInfo>,

    /// The user who created and sent the invitation.
    pub invitor_id: String,
    /// The qualified identifier of the user being invited.
    pub invitee_qid: String,

    /// Opaque token the invitee presents to accept the invitation.
    pub code: String,

    /// Whether this invitation has not yet been consumed.
    pub is_pending: bool,

    /// Role mask that will be assigned to the member upon acceptance.
    pub roles: RoleMask,
}

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
pub struct MemberInvitationUpdate {
    //
    /// Identifies which invitation record to update.
    pub id: String,
    /// Updated role mask to assign upon acceptance.
    pub roles: RoleMask,
}

/// Filtering, pagination, and include parameters for listing invitations.
pub struct MemberInvitationListSpec {
    //
    /// The team whose invitations should be listed.
    pub team_id: String,
    /// Consumption-status filter narrowing which invitations to return.
    pub status: MemberInvitationStatus,
    /// Additional data to include in each result, such as the inviter user record.
    pub incl_opt: Vec<MemberInvitationInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
