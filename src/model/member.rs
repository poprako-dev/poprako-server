//! Domain models for team membership.

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
