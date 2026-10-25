//! Review contents prepared for domain validation and replacement.

use crate::model::shared::issue::IssueRect;

/// One review issue before assigning a persisted identity and position.
pub struct IssueImport {
    /// Open issue category.
    pub variant: String,
    /// Optional normalized whole-page rectangle.
    pub rect: Option<IssueRect>,
    /// Original note text, preserved verbatim.
    pub note: String,
}

impl IssueImport {
    /// Borrows normalized variant without copying its storage.
    pub fn trimmed_variant(&self) -> &str {
        self.variant.trim()
    }
}

/// An explicit composite page and its ordered issues, including empty reviews.
pub struct PageIssueImport {
    /// Stable identity of the reviewed composite page.
    pub page_artwork_id: String,
    /// Issues in their intended persisted order.
    pub issues: Vec<IssueImport>,
}
