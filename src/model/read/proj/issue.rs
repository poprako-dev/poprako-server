//! Read-only projection of the chapter's current review issues.

use crate::model::shared::issue::IssueRect;

/// One ordered issue belonging to a Page in the current chapter review.
#[derive(Clone, Debug, PartialEq)]
pub struct IssueInfo {
    /// Server-generated identity, renewed on every replacement.
    pub id: String,

    /// Owning Page identity.
    pub page_id: String,
    /// Zero-based position within this Page.
    pub index: i32,

    /// Open, nonblank issue category.
    pub variant: String,
    /// Optional opaque layer path; absence targets the composite page.
    pub layer_path: Option<String>,
    /// Optional normalized rectangle in whole-page coordinates.
    pub rect: Option<IssueRect>,

    /// Original note, including whitespace, line breaks, and empty text.
    pub note: String,
}
