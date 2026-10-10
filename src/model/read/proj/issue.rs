//! Read-only projection of the chapter's current review issues.

use crate::model::shared::issue::IssueRect;

/// One ordered issue belonging to a composite page in the current chapter review.
#[derive(Clone, Debug, PartialEq)]
pub struct IssueInfo {
    /// Server-generated identity, renewed on every replacement.
    pub id: String,

    /// Owning composite page identity.
    pub page_artwork_id: String,
    /// Zero-based position within this review page.
    pub index: usize,

    /// Open, nonblank issue category.
    pub variant: String,
    /// Optional human-readable layer name; absence targets the composite page.
    pub layer_name: Option<String>,
    /// Optional normalized rectangle in whole-page coordinates.
    pub rect: Option<IssueRect>,

    /// Original note, including whitespace, line breaks, and empty text.
    pub note: String,
}
