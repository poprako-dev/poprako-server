//! Domain models for member invitations.

use poprako_macro::Paginate;

use crate::model::role::RoleMask;
use crate::model::user::UserInfo;
use crate::value::member_invitation::MemberInvitationInclOpt;

/// An invitation record for joining a team.
///
/// Carries an opaque invitation code, the inviter and invitee identifiers,
/// a pending flag indicating whether the invitation has been consumed,
/// and the [`RoleMask`] that will be assigned upon acceptance.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInvitationInfo {
    pub id: String,

    pub team_id: String,

    pub invitor: Option<UserInfo>,

    pub invitor_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,
}

/// The data needed to insert a member invitation row.
pub struct MemberInvitationForm {
    pub id: String,

    pub team_id: String,

    pub invitor_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub roles: RoleMask,
}

/// Mutable fields for a member invitation.
pub struct MemberInvitationUpdate {
    pub id: String,
    pub roles: RoleMask,
}

/// Filtering, pagination, and include parameters for listing invitations.
#[Paginate]
pub struct MemberInvitationListSpec {
    pub team_id: String,
    pub pending: Option<bool>,
    pub incl_opt: Vec<MemberInvitationInclOpt>,
}
