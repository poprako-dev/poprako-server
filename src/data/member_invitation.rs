//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::role::RoleMask;

/// Input parameters for creating a new team invitation.
///
/// The invitation binds a specific QQ ID (`invitee_qid`) to a [`RoleMask`]
/// that will be granted upon acceptance. The actual in-app user lookup
/// happens during the registration flow.
pub struct CreateMemberInvitationData {
    pub team_id: String,

    /// The QQ ID of the person being invited (not a user UUID).
    pub invitee_qid: String,

    /// The role mask that will be assigned when the invitee registers
    /// and accepts the invitation.
    pub role_mask: RoleMask,
}

/// Return value from a successful invitation creation.
///
/// The `code` is a short opaque token the invitee presents during
/// registration to claim the invitation.
pub struct CreateMemberInvitationVal {
    pub id: String,
    pub code: String,
}

/// Input parameters for listing invitations within a team, with optional
/// pending-status filtering and standard offset/limit pagination.
pub struct ListMemberInvitationInfosData {
    pub team_id: String,

    /// When `Some(true)`, returns only unconsumed invitations;
    /// `Some(false)` returns only consumed ones; `None` returns all.
    pub pending: Option<bool>,

    pub offset: u64,
    pub limit: u64,
}

/// Presentation-ready member invitation information.
///
/// Mirrors [`MemberInvitationInfo`] with timestamps omitted (the domain
/// model carries no timestamps).
///
/// [`MemberInvitationInfo`]: crate::model::member_invitation::MemberInvitationInfo
pub struct MemberInvitationInfoVal {
    pub id: String,

    pub team_id: String,

    pub invitor_id: String,

    pub invitee_qid: String,
    pub code: String,

    pub pending: bool,

    pub role_mask: RoleMask,
}

impl From<MemberInvitationInfo> for MemberInvitationInfoVal {
    fn from(value: MemberInvitationInfo) -> Self {
        Self {
            id: value.id,
            team_id: value.team_id,
            invitor_id: value.invitor_id,
            invitee_qid: value.invitee_qid,
            code: value.code,
            pending: value.pending,
            role_mask: value.role_mask,
        }
    }
}

/// Input parameters for updating a pending invitation's role mask.
pub struct UpdateMemberInvitationInfoData {
    pub id: String,
    pub role_mask: RoleMask,
}
