//! Result of replacing a chapter's current review.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Counts for the complete chapter import, including empty Pages.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterIssuesVal {
    /// Number of mapped Pages.
    pub imported_page_count: usize,
    /// Number of newly inserted issues.
    pub imported_issue_count: usize,
}
