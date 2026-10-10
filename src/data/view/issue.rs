//! Read-only issue and geometry presentation.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::shared::issue::IssueRect;

/// Geometry in normalized whole-page coordinates.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueRectView {
    /// Left edge.
    pub x_coord: f64,
    /// Top edge.
    pub y_coord: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl From<IssueRect> for IssueRectView {
    // Converts the domain value into its corresponding representation.
    fn from(rect: IssueRect) -> Self {
        Self {
            x_coord: rect.x_coord,
            y_coord: rect.y_coord,
            width: rect.width,
            height: rect.height,
        }
    }
}

/// Read-only issue in the chapter's single current review.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueInfoView {
    /// Server-generated identity.
    pub id: String,

    /// Owning composite page identity.
    pub page_artwork_id: String,
    /// Zero-based position within the review page.
    pub index: usize,

    /// Open category.
    pub variant: String,
    /// Optional human-readable layer name, preserved verbatim.
    pub layer_name: Option<String>,
    /// Optional whole-page rectangle.
    pub rect: Option<IssueRectView>,

    /// Original note text.
    pub note: String,
}

impl From<IssueInfo> for IssueInfoView {
    // Converts the domain value into its corresponding representation.
    fn from(info: IssueInfo) -> Self {
        Self {
            id: info.id,
            page_artwork_id: info.page_artwork_id,
            index: info.index,
            variant: info.variant,
            layer_name: info.layer_name,
            rect: info.rect.map(Into::into),
            note: info.note,
        }
    }
}
