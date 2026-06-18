use crate::model::role::RoleMask;

pub struct MemberInfo {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
}

pub struct MemberForm {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
    pub role_mask: RoleMask,
}
