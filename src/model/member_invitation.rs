use crate::model::role::RoleMask;

#[cfg_attr(test, derive(Clone))]
pub struct MemberInvitationInfo {
    pub id: String,
    pub team_id: String,
    pub invitor_id: String,
    pub invitee_qid: String,
    pub code: String,
    pub pending: bool,
    pub role_mask: RoleMask,
}
