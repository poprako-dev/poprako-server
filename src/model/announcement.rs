//! Domain models for team announcements.

use time::OffsetDateTime;

use poprako_macro::Paginate;

use crate::model::user::UserInfo;
use crate::value::announcement::AnnouncementInclOpt;

/// A team announcement as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementInfo {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    pub user: Option<UserInfo>,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,
}

/// The data needed to insert a team announcement row.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementForm {
    pub id: String,

    pub team_id: String,
    pub user_id: String,

    pub title: String,
    pub content: String,
}

/// Filtering, pagination, and include parameters for listing announcements.
#[Paginate]
pub struct AnnouncementListSpec {
    pub team_id: String,
    pub incl_opt: Vec<AnnouncementInclOpt>,
}
