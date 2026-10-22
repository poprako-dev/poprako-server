//! Domain models for team membership.

use crate::value::member::MemberInclOpt;
use crate::value::pagination::PubListLimit;
use crate::value::role::RoleField;

/// Filtering and pagination parameters for listing memberships.
pub enum MemberListSpec<'a> {
    /// List teams/memberships owned by a specific user.
    User {
        /// ID of the user whose memberships to list.
        owner_id: &'a str,
        /// Optional include flags for related data.
        incl_opt: &'a [MemberInclOpt],
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },

    /// List members of a specific team, optionally narrowed by role or nickname.
    Team {
        /// ID of the team whose members to list.
        team_id: &'a str,
        /// Fuzzy filter by member nickname.
        fuzzy_nickname: Option<&'a str>,
        /// Optional role filter.
        role: Option<RoleField>,
        /// Optional include flags for related data.
        incl_opt: &'a [MemberInclOpt],
        /// Number of records to skip for pagination.
        offset: u32,
        /// Maximum number of records to return.
        limit: PubListLimit,
    },
}

impl MemberListSpec<'_> {
    /// Returns the include options regardless of which variant the spec is.
    pub const fn incl_opt(&self) -> &[MemberInclOpt] {
        //
        match self {
            //
            Self::User { incl_opt, .. } | Self::Team { incl_opt, .. } => {
                incl_opt
            }
        }
    }
}
