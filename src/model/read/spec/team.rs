//! Domain models for team profile storage.

/// Filtering and pagination parameters for listing teams.
pub struct TeamListSpec {
    /// Optional user whose team memberships scope the listing.
    pub user_id: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}
