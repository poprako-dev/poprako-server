//! Data transfer objects for member use cases.

use poprako_macro::Paginate;

use crate::model::member::MemberInfo;

/// Presentation-ready membership information.
pub struct MemberInfoVal {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub role_mask: u32,
}

impl From<MemberInfo> for MemberInfoVal {
    fn from(value: MemberInfo) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            user_nickname: value.user_nickname,
            team_id: value.team_id,
            role_mask: value.role_mask.0,
        }
    }
}

/// Input parameters for creating a member.
pub struct CreateMemberData {
    pub user_id: String,
    pub team_id: String,

    pub role_mask: u32,
}

/// Return value from creating a member.
pub struct CreateMemberVal {
    pub id: String,
}

/// Input parameters for listing members by team.
#[Paginate]
pub struct ListMemberInfosData {
    pub team_id: String,

    pub user_nickname_keyword: Option<String>,
    pub role_mask: Option<u32>,
}

/// Input parameters for listing memberships of the current user.
#[Paginate]
pub struct ListMineMemberInfosData {}

/// Input parameters for updating a member role mask.
pub struct UpdateMemberRoleData {
    pub id: String,
    pub role_mask: u32,
}
