//! Domain models for team board comments.

use time::OffsetDateTime;

use crate::model::user::UserInfo;
use crate::value::comment::CommentInclOpt;

/// A team board comment as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct CommentInfo {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    pub user: Option<UserInfo>,

    pub content: String,

    pub created_at: OffsetDateTime,
}

/// The data needed to insert a team board comment row.
#[cfg_attr(test, derive(Clone))]
pub struct CommentEntry {
    pub id: String,

    pub team_id: String,
    pub user_id: String,

    pub content: String,
}

/// Filtering, pagination, and include parameters for listing comments.
pub struct CommentListSpec {
    pub team_id: String,
    pub incl_opt: Vec<CommentInclOpt>,

    pub offset: u32,
    pub limit: u32,
}
