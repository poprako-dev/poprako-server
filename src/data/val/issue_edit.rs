//! Consistent page documents and chapter category operation results.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::issue_edit::{IssueEditVariantCountView, IssueEditView};

/// Editable contents tied to a specific review and image generation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
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
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,

    /// Ordered annotations, excluding legacy layer metadata.
    pub issues: Vec<IssueEditView>,
}

/// Stored category references, never the user's local presets.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditVariantsVal {
    /// Opaque review identity.
    pub review_key: String,
    /// Decimal chapter revision.
    pub revision: String,
    /// Names currently used by at least one annotation.
    pub variants: Vec<IssueEditVariantCountView>,
}

/// Result of an atomic chapter-wide category rename.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RenameIssueEditVariantVal {
    /// Current usage summary after the rename.
    pub summary: IssueEditVariantsVal,
    /// Fresh documents for every changed page.
    pub pages: Vec<IssueEditPageVal>,
}
