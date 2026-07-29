//! Domain models for team board comments.

use time::OffsetDateTime;

use crate::model::user::UserInfo;
use crate::value::comment::CommentInclOpt;

/// A team board comment as stored in the database, with optional included user data.
#[cfg_attr(test, derive(Clone))]
pub struct CommentInfo {
    //
    /// Unique identifier for the comment.
    pub id: String,

    /// The team board this comment belongs to.
    pub team_id: String,
    /// The user who authored the comment.
    pub user_id: String,
    /// The resolved user record, populated when the include option is set.
    pub user: Option<UserInfo>,

    /// The text body of the comment.
    pub content: String,

    /// When this comment was first created.
    pub created_at: OffsetDateTime,
}

/// The data needed to insert a team board comment row.
#[cfg_attr(test, derive(Clone))]
pub struct CommentEntry {
    //
    /// Unique identifier for the new comment.
    pub id: String,

    /// The team board this comment is being posted to.
    pub team_id: String,
    /// The user who is creating this comment.
    pub user_id: String,

    /// The text body of the comment being posted.
    pub content: String,
}

/// Filtering, pagination, and include parameters for listing comments.
pub struct CommentListSpec {
    //
    /// The team whose board comments should be listed.
    pub team_id: String,
    /// Additional data to include in each result, such as the author user record.
    pub incl_opt: Vec<CommentInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
