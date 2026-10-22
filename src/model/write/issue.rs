//! Complete replacement parameters for the single current chapter review.

use crate::model::shared::issue::IssueRect;

/// One issue to insert after validating the entire chapter input.
pub struct IssueEntry<'a> {
    /// New server-generated identity.
    pub id: String,

    /// Owning composite page identity.
    pub page_artwork_id: &'a str,
    /// Contiguous zero-based position within the review page.
    pub index: usize,

    /// Open issue category.
    pub variant: &'a str,
    /// Optional human-readable layer name, preserved verbatim.
    pub layer_name: Option<&'a str>,
    /// Optional whole-page rectangle.
    pub rect: Option<IssueRect>,

    /// Original note text.
    pub note: &'a str,
}

/// Atomic replacement of all issues in one Chapter, including empty review pages.
pub struct ChapterIssuesRepl<'a> {
    /// Chapter whose current review is replaced.
    pub chapter_id: &'a str,
    /// Validated issues in page and issue order.
    pub entries: &'a [IssueEntry<'a>],
}
