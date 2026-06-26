//! Domain models for team membership.

use poprako_macro::Paginate;

use crate::model::role::RoleMask;

/// A membership record linking a user to a team.
///
/// Lightweight projection — does not carry the full user or team record,
/// only the identifiers and a cached nickname for display purposes.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInfo {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub role_mask: RoleMask,
}

/// The data needed to insert a new membership row.
///
/// Includes a [`RoleMask`] specifying the member's permissions within the team.
#[cfg_attr(test, derive(Clone))]
pub struct MemberForm {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub role_mask: RoleMask,
}

/// Mutable fields for a membership record.
pub struct MemberRoleUpdate {
    pub id: String,
    pub role_mask: RoleMask,
}

/// Filtering and pagination parameters for listing team members.
#[Paginate]
pub struct MemberListSpec {
    pub team_id: String,

    pub user_nickname_keyword: Option<String>,
    pub role_mask: Option<RoleMask>,
}
