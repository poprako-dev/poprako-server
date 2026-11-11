//! Result of replacing a chapter's current review.
/// Counts for the complete chapter import, including empty review pages.
pub struct ImportChapterIssuesVal {
    /// Number of pages in the imported review file.
    pub imported_page_count: usize,
    /// Number of newly inserted issues.
    pub imported_issue_count: usize,
}
