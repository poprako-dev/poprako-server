//! Domain models for team membership.

use time::OffsetDateTime;

use crate::model::team_model;
use crate::model::user_model;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

/// A membershiprecord linking a user to a team.
///
/// Lightweight projection — does not carry the full user or team record,
/// only the identifiers and a cached nickname for display purposes.
#[cfg_attr(test, derive(Clone))]
pub struct Info {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,
    pub user_last_active_at: OffsetDateTime,

    pub team_id: String,

    pub user: Option<user_model::Info>,
    pub team: Option<team_model::Info>,

    pub roles: RoleMask,
}

/// The data needed to insert a new membership row.
///
/// Includes a [`RoleMask`] specifying the member's permissions within the team.
#[cfg_attr(test, derive(Clone))]
pub struct Form {
    pub id: String,

    pub user_id: String,
    pub user_nickname: String,

    pub team_id: String,

    pub roles: RoleMask,
}

/// Mutable fields for a membership record.
pub struct RoleUpdate {
    pub id: String,
    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing memberships.
pub enum ListSpec {
    User {
        owner_id: String,
        incl_opt: Vec<MemberInclOpt>,
        offset: u64,
        limit: u64,
    },
    Team {
        team_id: String,
        fuzzy_nickname: Option<String>,
        role: Option<RoleField>,
        incl_opt: Vec<MemberInclOpt>,
        offset: u64,
        limit: u64,
    },
}

impl ListSpec {
    /// Returns the include options regardless of which variant the spec is.
    pub fn incl_opt(&self) -> &[MemberInclOpt] {
        match self {
            //
            ListSpec::User { incl_opt, .. } => incl_opt,

            ListSpec::Team { incl_opt, .. } => incl_opt,
        }
    }
}
