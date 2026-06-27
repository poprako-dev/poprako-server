//! Domain models for team membership.

use crate::model::role::{RoleBit, RoleMask};
use crate::value::member::MemberInclOpt;

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

/// Filtering and pagination parameters for listing memberships.
pub enum MemberListSpec {
    User {
        owner_id: String,
        incl_opt: Vec<MemberInclOpt>,
        offset: u64,
        limit: u64,
    },
    Team {
        team_id: String,
        role_bit: Option<RoleBit>,
        incl_opt: Vec<MemberInclOpt>,
        offset: u64,
        limit: u64,
    },
}
