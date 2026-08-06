//! Domain models for team announcements.

use crate::value::announcement::AnnouncementInclOpt;

/// Filtering, pagination, and include parameters for listing announcements.
pub struct AnnouncementListSpec {
    /// Scopes the listing to announcements within this team.
    pub team_id: String,
    /// Flags controlling which optional associations (such as user data) are
    /// joined into results.
    pub incl_opt: Vec<AnnouncementInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
