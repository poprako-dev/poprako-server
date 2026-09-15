//! Domain models for team board comments.

/// The data needed to insert a team board comment row.
#[cfg_attr(test, derive(Clone))]
pub struct CommentEntry {
    /// Unique identifier for the new comment.
    pub id: String,

    /// The team board this comment is being posted to.
    pub team_id: String,
    /// The user who is creating this comment.
    pub user_id: String,

    /// The text body of the comment being posted.
    pub content: String,
}
