//! Domain models for member invitations.

use poprako_macro::Paginate;

use crate::model::user_model;
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

/// An invitation record for joining a team.
///
/// Carries an opaque invitation code, the inviter and invitee identifiers,
/// a pending flag indicating whether the invitation has been consumed,
/// and the [`RoleMask`] that will be assigned upon acceptance.
#[cfg_attr(test, derive(Clone))]
pub struct Info {
    pub id: String,

    pub team_id: String,

    pub invitor: Option<user_model::Info>,

    pub invitor_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub pending: bool,

    pub roles: RoleMask,
}

/// The data needed to insert a member invitation row.
pub struct Form {
    pub id: String,

    pub team_id: String,

    pub invitor_id: String,
    pub invitee_qid: String,

    pub code: String,

    pub roles: RoleMask,
}

/// Mutable fields for a member invitation.
pub struct Update {
    pub id: String,
    pub roles: RoleMask,
}

/// Filtering, pagination, and include parameters for listing invitations.
#[Paginate]
pub struct ListSpec {
    pub team_id: String,
    pub pending: Option<bool>,
    pub incl_opt: Vec<MemberInvitationInclOpt>,
}
