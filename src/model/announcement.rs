//! Domain models for team announcements.

use time::OffsetDateTime;

use crate::model::user::UserInfo;
use crate::value::announcement::AnnouncementInclOpt;

/// A team announcement as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementInfo {
    /// Unique identifier for the announcement row.
    pub id: String,

    /// Foreign key to the team this announcement belongs to.
    pub team_id: String,
    /// Foreign key to the user who created this announcement.
    pub user_id: String,
    /// Optional joined user data included when the query specifies user expansion.
    pub user: Option<UserInfo>,

    /// Headline text of the announcement.
    pub title: String,
    /// Full body text of the announcement.
    pub content: String,

    /// Timestamp when the announcement was posted.
    pub created_at: OffsetDateTime,
}

/// The data needed to insert a team announcement row.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementEntry {
    /// Unique identifier to insert for the new announcement row.
    pub id: String,

    /// Foreign key identifying the target team for this announcement.
    pub team_id: String,
    /// Foreign key identifying the posting user.
    pub user_id: String,

    /// Headline text for the new announcement.
    pub title: String,
    /// Full body text for the new announcement.
    pub content: String,
}

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
