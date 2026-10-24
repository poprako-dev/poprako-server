//! Layer-independent annotation presentation for the editor.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::issue::IssueRectView;
use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::issue_edit::IssueEditVariantCount;

/// One editable page annotation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditView {
    /// Stable identity.
    pub id: String,

    /// Owning composite page.
    pub page_artwork_id: String,
    /// Zero-based position.
    pub index: usize,

    /// Category name.
    pub variant: String,
    /// Null denotes a whole-page annotation.
    pub rect: Option<IssueRectView>,
    /// Original note text.
    pub note: String,
}

impl From<IssueInfo> for IssueEditView {
    // Converts the internal projection to its wire view.
    fn from(info: IssueInfo) -> Self {
        Self {
            id: info.id,
            page_artwork_id: info.page_artwork_id,
            index: info.index,
            variant: info.variant,
            rect: info.rect.map(Into::into),
            note: info.note,
        }
    }
}

/// Usage of a category within the current chapter.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditVariantCountView {
    /// Category name.
    pub variant: String,
    /// Number of stored references.
    pub count: i64,
}

impl From<IssueEditVariantCount> for IssueEditVariantCountView {
    // Converts the internal projection to its wire view.
    fn from(info: IssueEditVariantCount) -> Self {
        Self {
            variant: info.variant,
            count: info.count,
        }
    }
}
