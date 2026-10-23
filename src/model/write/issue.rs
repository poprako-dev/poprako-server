//! Complete replacement parameters for the single current chapter review.

use crate::model::shared::issue::IssueRect;

/// One issue to insert after validating the entire chapter input.
#[cfg_attr(test, derive(Clone))]
pub struct IssueEntry {
    /// New server-generated identity.
    pub id: String,

    /// Owning composite page identity.
    pub page_artwork_id: String,
    /// Contiguous zero-based position within the review page.
    pub index: usize,

    /// Open issue category.
    pub variant: String,
    /// Optional human-readable layer name, preserved verbatim.
    pub layer_name: Option<String>,
    /// Optional whole-page rectangle.
    pub rect: Option<IssueRect>,

    /// Original note text.
    pub note: String,
}

impl IssueEntry {
    /// Borrows normalized variant without copying its storage.
    pub fn trimmed_variant(&self) -> &str {
        self.variant.trim()
    }
}

/// Atomic replacement of all issues in one Chapter, including empty review pages.
pub struct ChapterIssuesRepl {
    /// Chapter whose current review is replaced.
    pub chapter_id: String,
    /// Validated issues in page and issue order.
    pub entries: Vec<IssueEntry>,
}
