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
    /// Unique identifier for the membership record.
    pub id: String,

    /// The user who holds this membership.
    pub user_id: String,
    /// Display nickname cached from the user record at membership time.
    pub user_nickname: String,
    /// Timestamp of the user's most recent activity, cached for quick sorting.
    pub user_last_active_at: OffsetDateTime,

    /// The team this membership belongs to.
    pub team_id: String,

    /// The resolved user record, populated when the include option is set.
    pub user: Option<UserInfo>,
    /// The resolved team record, populated when the include option is set.
    pub team: Option<TeamInfo>,

    /// Bitmask of roles and permissions assigned to this member within the team.
    pub roles: RoleMask,
}

/// The data needed to insert a new membership row.
///
/// Includes a [`RoleMask`] specifying the member's permissions within the team.
#[cfg_attr(test, derive(Clone))]
pub struct MemberEntry {
    /// Unique identifier for the new membership record.
    pub id: String,

    /// The user who will be granted membership.
    pub user_id: String,
    /// Display nickname to cache from the user record at insertion time.
    pub user_nickname: String,

    /// The team the user is joining.
    pub team_id: String,

    /// Bitmask of roles and permissions assigned to this member.
    pub roles: RoleMask,
}

/// Mutable fields for a membership record.
pub struct MemberRoleUpdate {
    /// Identifies which membership record to update.
    pub id: String,
    /// Updated bitmask of roles and permissions for the member.
    pub roles: RoleMask,
}

/// Filtering and pagination parameters for listing memberships.
pub enum MemberListSpec {
    /// List teams/memberships owned by a specific user.
    User {
        /// ID of the user whose memberships to list.
        owner_id: String,
        /// Optional include flags for related data.
        incl_opt: Vec<MemberInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: u32,
    },
    /// List members of a specific team, optionally narrowed by role or nickname.
    Team {
        /// ID of the team whose members to list.
        team_id: String,
        /// Fuzzy filter by member nickname.
        fuzzy_nickname: Option<String>,
        /// Optional role filter.
        role: Option<RoleField>,
        /// Optional include flags for related data.
        incl_opt: Vec<MemberInclOpt>,
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
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
