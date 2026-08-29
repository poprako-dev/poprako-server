//! Query specification for chapter workflow record listings.

/// Pagination and ownership filter for chapter workflow records.
pub struct ChapterWorkflowRecordListSpec {
    /// Chapter whose records are listed.
    pub chapter_id: String,
    /// Number of records to skip.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
