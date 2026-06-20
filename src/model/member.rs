use crate::model::role::RoleMask;

#[cfg_attr(test, derive(Clone))]
pub struct MemberInfo {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
}

#[cfg_attr(test, derive(Clone))]
pub struct MemberForm {
    pub id: String,
    pub user_id: String,
    pub user_nickname: String,
    pub team_id: String,
    pub role_mask: RoleMask,
}
