use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::role::RoleMask;

pub struct CreateMemberInvitationData {
    pub team_id: String,

    pub invitee_qid: String,

    pub role_mask: RoleMask,
}

pub struct CreateMemberInvitationVal {
    pub id: String,
    pub code: String,
}

pub struct ListMemberInvitationInfosData {
    pub team_id: String,

    pub pending: Option<bool>,

    pub offset: u64,
    pub limit: u64,
}

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

pub struct UpdateMemberInvitationInfoData {
    pub id: String,
    pub role_mask: RoleMask,
}
