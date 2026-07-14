//! Domain models for team membership.

use time::OffsetDateTime;

use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// A membershiprecord linking a user to a team.
///
/// Lightweight projection — does not carry the full user or team record,
/// only the identifiers and a cached nickname for display purposes.
#[cfg_attr(test, derive(Clone))]
pub struct MemberInfo {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user_last_active_at: OffsetDateTime,

    pub team_id: String,

    pub user: Option<UserInfo>,
    pub team: Option<TeamInfo>,

    pub roles: RoleMask,
}

/// The data needed to insert a new membership row.
///
/// Includes a [`RoleMask`] specifying the member's permissions within the team.
#[cfg_attr(test, derive(Clone))]
pub struct MemberEntry {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,
}

/// Mutable fields for a membership record.
pub struct MemberRoleUpdate {
    pub id: String,
    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing memberships.
pub enum MemberListSpec {
    /// List teams/memberships owned by a specific user.
    User {
        owner_id: String,
        incl_opt: Vec<MemberInclOpt>,
        offset: u32,
        limit: u32,
    },
    /// List members of a specific team, optionally narrowed by role or nickname.
    Team {
        team_id: String,
        fuzzy_nickname: Option<String>,
        role: Option<RoleField>,
        incl_opt: Vec<MemberInclOpt>,
        offset: u32,
        limit: u32,
    },
}

impl MemberListSpec {
    /// Returns the include options regardless of which variant the spec is.
    pub fn incl_opt(&self) -> &[MemberInclOpt] {
        match self {
            //
            MemberListSpec::User { incl_opt, .. } => incl_opt,

            MemberListSpec::Team { incl_opt, .. } => incl_opt,
        }
    }
}
