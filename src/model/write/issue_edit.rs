//! Page replacement and chapter category rename commands.

use crate::model::write::issue::IssueEntry;

/// Complete final contents of a single page.
pub struct IssueEditPageRepl {
    /// Owning chapter, already locked by the caller.
    pub chapter_id: String,
    /// Target composite page.
    pub page_artwork_id: String,
    /// Validated ordered contents.
    pub entries: Vec<IssueEntry>,
}

/// Atomic rename of current chapter references.
pub struct IssueEditVariantRename {
    /// Chapter whose references may change.
    pub chapter_id: String,
    /// Existing normalized name.
    pub from: String,
    /// Replacement normalized name.
    pub to: String,
}
