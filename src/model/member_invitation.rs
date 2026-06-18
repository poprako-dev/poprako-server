use crate::model::role::RoleMask;

pub struct MemberInvitationInfo {
    pub id: String,
    pub team_id: String,
    pub invitor_id: String,
    pub invitee_qid: String,
    pub role_mask: RoleMask,
}
