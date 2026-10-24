//! Version-checked annotation editing requests.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Rectangle expressed in whole-page normalized coordinates.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditRectInstr {
    /// Left edge.
    pub x_coord: f64,
    /// Top edge.
    pub y_coord: f64,
    /// Positive width.
    pub width: f64,
    /// Positive height.
    pub height: f64,
}

/// Final contents of one stable annotation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueEditEntryInstr {
    /// Stable client UUID or existing imported identity.
    pub id: String,

    /// Nonblank category name.
    pub variant: String,
    /// Null denotes a whole-page annotation.
    pub rect: Option<IssueEditRectInstr>,
    /// Original text, including empty strings and whitespace.
    pub note: String,
}

/// Complete ordered page replacement guarded by its loaded context.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageIssuesInstr {
    /// Opaque review identity returned by the read endpoint.
    pub review_key: String,
    /// Decimal page revision returned by the read endpoint.
    pub revision: String,
    /// Exact image generation shown while editing, or null.
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,

    /// Final ordered contents; empty clears the page.
    pub issues: Vec<IssueEditEntryInstr>,
}

/// Renames references only when the chapter snapshot is still current.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RenameIssueEditVariantInstr {
    /// Expected review identity.
    pub review_key: String,
    /// Decimal chapter revision from the category summary.
    pub revision: String,

    /// Current category name.
    pub from: String,
    /// New category name.
    pub to: String,
}
