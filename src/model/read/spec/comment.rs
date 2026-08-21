//! Domain models for team board comments.

use crate::value::comment::CommentInclOpt;

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
