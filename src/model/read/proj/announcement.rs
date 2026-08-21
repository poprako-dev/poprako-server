//! Domain models for team announcements.

use time::OffsetDateTime;

use crate::model::read::proj::user::UserInfo;

/// A team announcement as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementInfo {
    //
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
