//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::issue::IssueRectObj;
use crate::api::http::result::Wire;
use crate::data::instr::issue_edit::{
    IssueEditEntryInstr, IssueEditRectInstr, RenameIssueEditVariantInstr,
    SavePageIssuesInstr,
};
use crate::data::val::issue_edit::{
    CreatedIssueIdVal, IssueEditPageVal, IssueEditVariantsVal,
    RenameIssueEditVariantVal, SavePageIssuesVal,
};
use crate::data::view::issue_edit::{IssueEditVariantCountView, IssueEditView};

/// Rectangle expressed in whole-page normalized coordinates.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditRectBody {
    /// Left edge.
    pub x_coord: f64,
    /// Top edge.
    pub y_coord: f64,
    /// Positive width.
    pub width: f64,
    /// Positive height.
    pub height: f64,
}

impl From<IssueEditRectBody> for IssueEditRectInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditRectBody) -> Self {
        Self {
            x_coord: value.x_coord,
            y_coord: value.y_coord,
            width: value.width,
            height: value.height,
        }
    }
}

/// Final contents of one stable annotation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditEntryBody {
    /// Existing server-assigned identity; exclusive with `local_id`.
    pub id: Option<String>,
    /// Temporary identity for a new annotation; exclusive with id.
    pub local_id: Option<String>,

    /// Nonblank category name.
    pub variant: String,
    /// Null denotes a whole-page annotation.
    pub rect: Option<IssueEditRectBody>,
    /// Original text, including empty strings and whitespace.
    pub note: String,
}

impl From<IssueEditEntryBody> for IssueEditEntryInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditEntryBody) -> Self {
        Self {
            id: value.id,
            local_id: value.local_id,
            variant: value.variant,
            rect: value.rect.map(std::convert::Into::into),
            note: value.note,
        }
    }
}

/// Complete ordered page replacement guarded by its loaded context.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageIssuesBody {
    /// Opaque review identity returned by the read endpoint.
    pub review_key: String,
    /// Decimal page revision returned by the read endpoint.
    pub revision: String,
    /// Exact image generation shown while editing, or null.
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,

    /// Final ordered contents; empty clears the page.
    pub issues: Vec<IssueEditEntryBody>,
}

impl From<SavePageIssuesBody> for SavePageIssuesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SavePageIssuesBody) -> Self {
        Self {
            review_key: value.review_key,
            revision: value.revision,
            image_ver: value.image_ver,
            issues: value
                .issues
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Renames references only when the chapter snapshot is still current.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RenameIssueEditVariantBody {
    /// Expected review identity.
    pub review_key: String,
    /// Decimal chapter revision from the category summary.
    pub revision: String,

    /// Current category name.
    pub from: String,
    /// New category name.
    pub to: String,
}

impl From<RenameIssueEditVariantBody> for RenameIssueEditVariantInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: RenameIssueEditVariantBody) -> Self {
        Self {
            review_key: value.review_key,
            revision: value.revision,
            from: value.from,
            to: value.to,
        }
    }
}

/// Editable contents tied to a specific review and image generation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditPagePayload {
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

    /// Ordered annotations.
    pub issues: Vec<IssueEditObj>,
}

impl From<IssueEditPageVal> for IssueEditPagePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditPageVal) -> Self {
        Self {
            page_artwork_id: value.page_artwork_id,
            review_key: value.review_key,
            revision: value.revision,
            chapter_revision: value.chapter_revision,
            image_ver: value.image_ver,
            issues: value
                .issues
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for IssueEditPageVal {
    // Serializable wire payload produced by this conversion.
    type Target = IssueEditPagePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Stored category references, never the user's local presets.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditVariantsPayload {
    /// Opaque review identity.
    pub review_key: String,
    /// Decimal chapter revision.
    pub revision: String,
    /// Names currently used by at least one annotation.
    pub variants: Vec<IssueEditVariantCountObj>,
}

impl From<IssueEditVariantsVal> for IssueEditVariantsPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditVariantsVal) -> Self {
        Self {
            review_key: value.review_key,
            revision: value.revision,
            variants: value
                .variants
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for IssueEditVariantsVal {
    // Serializable wire payload produced by this conversion.
    type Target = IssueEditVariantsPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Result of an atomic chapter-wide category rename.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RenameIssueEditVariantPayload {
    /// Current usage summary after the rename.
    pub summary: IssueEditVariantsPayload,
    /// Fresh documents for every changed page.
    pub pages: Vec<IssueEditPagePayload>,
}

impl From<RenameIssueEditVariantVal> for RenameIssueEditVariantPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: RenameIssueEditVariantVal) -> Self {
        Self {
            summary: value.summary.into(),
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for RenameIssueEditVariantVal {
    // Serializable wire payload produced by this conversion.
    type Target = RenameIssueEditVariantPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// One new annotation's client-to-server identity mapping.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreatedIssueIdPayload {
    /// Temporary identity submitted by the client.
    pub local_id: String,
    /// Persistent identity allocated by the server.
    pub issue_id: String,
}

impl From<CreatedIssueIdVal> for CreatedIssueIdPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: CreatedIssueIdVal) -> Self {
        Self {
            local_id: value.local_id,
            issue_id: value.issue_id,
        }
    }
}

impl Wire for CreatedIssueIdVal {
    // Serializable wire payload produced by this conversion.
    type Target = CreatedIssueIdPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Committed annotation document and identities assigned by the save batch.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageIssuesPayload {
    /// Latest document under the same review and image context.
    pub page: IssueEditPagePayload,
    /// Stable mappings retained for retries of this batch.
    pub created_issue_ids: Vec<CreatedIssueIdPayload>,
}

impl From<SavePageIssuesVal> for SavePageIssuesPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: SavePageIssuesVal) -> Self {
        Self {
            page: value.page.into(),
            created_issue_ids: value
                .created_issue_ids
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for SavePageIssuesVal {
    // Serializable wire payload produced by this conversion.
    type Target = SavePageIssuesPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// One editable page annotation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditObj {
    /// Stable identity.
    pub id: String,

    /// Owning composite page.
    pub page_artwork_id: String,
    /// Zero-based position.
    pub index: usize,

    /// Category name.
    pub variant: String,
    /// Null denotes a whole-page annotation.
    pub rect: Option<IssueRectObj>,
    /// Original note text.
    pub note: String,
}

impl From<IssueEditView> for IssueEditObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditView) -> Self {
        Self {
            id: value.id,
            page_artwork_id: value.page_artwork_id,
            index: value.index,
            variant: value.variant,
            rect: value.rect.map(std::convert::Into::into),
            note: value.note,
        }
    }
}

impl Wire for IssueEditView {
    // Serializable wire payload produced by this conversion.
    type Target = IssueEditObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Usage of a category within the current chapter.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditVariantCountObj {
    /// Category name.
    pub variant: String,
    /// Number of stored references.
    pub count: i64,
}

impl From<IssueEditVariantCountView> for IssueEditVariantCountObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueEditVariantCountView) -> Self {
        Self {
            variant: value.variant,
            count: value.count,
        }
    }
}

impl Wire for IssueEditVariantCountView {
    // Serializable wire payload produced by this conversion.
    type Target = IssueEditVariantCountObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Immutable request identity reused only when retrying the same batch.
#[derive(Deserialize)]
pub struct SaveQuery {
    /// UUID batch identity.
    pub save_id: String,
}
