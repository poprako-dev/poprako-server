//! Consistent page documents and chapter category operation results.
use crate::data::view::issue_edit::{IssueEditVariantCountView, IssueEditView};

/// Editable contents tied to a specific review and image generation.
pub struct IssueEditPageVal {
    /// Owning composite page.
    pub page_artwork_id: String,

    /// Opaque whole-review identity.
    pub review_key: String,
    /// Decimal page revision.
    pub revision: String,
    /// Decimal chapter revision.
    pub chapter_revision: String,
    /// Current allocated image generation.
    pub image_ver: Option<u32>,

    /// Ordered annotations.
    pub issues: Vec<IssueEditView>,
}

/// Stored category references, never the user's local presets.
pub struct IssueEditVariantsVal {
    /// Opaque review identity.
    pub review_key: String,
    /// Decimal chapter revision.
    pub revision: String,
    /// Names currently used by at least one annotation.
    pub variants: Vec<IssueEditVariantCountView>,
}

/// Result of an atomic chapter-wide category rename.
pub struct RenameIssueEditVariantVal {
    /// Current usage summary after the rename.
    pub summary: IssueEditVariantsVal,
    /// Fresh documents for every changed page.
    pub pages: Vec<IssueEditPageVal>,
}

/// One new annotation's client-to-server identity mapping.
pub struct CreatedIssueIdVal {
    /// Temporary identity submitted by the client.
    pub local_id: String,
    /// Persistent identity allocated by the server.
    pub issue_id: String,
}

/// Committed annotation document and identities assigned by the save batch.
pub struct SavePageIssuesVal {
    /// Latest document under the same review and image context.
    pub page: IssueEditPageVal,
    /// Stable mappings retained for retries of this batch.
    pub created_issue_ids: Vec<CreatedIssueIdVal>,
}
