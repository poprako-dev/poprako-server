//! Domain models for team membership.

use crate::value::member::MemberInclOpt;
use crate::value::role::RoleField;

/// Filtering and pagination parameters for listing memberships.
pub enum MemberListSpec {
    //
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
        //
        match self {
            //
            MemberListSpec::User { incl_opt, .. } => incl_opt,

            MemberListSpec::Team { incl_opt, .. } => incl_opt,
        }
    }
}
