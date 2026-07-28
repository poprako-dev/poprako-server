//! Domain models for team profile storage.

/// Filtering and pagination parameters for listing teams.
pub struct TeamListSpec {
    //
    /// Membership filter mode for the team listing.
    pub kind: TeamListKind,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}

/// Membership filtering mode for listing teams.
pub enum TeamListKind {
    //
    /// Include all teams.
    All,

    /// Include only teams joined by the specified user.
    JoinedBy {
        /// ID of the user whose team memberships to list.
        user_id: String,
    },
}
