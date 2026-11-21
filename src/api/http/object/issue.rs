//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::result::Wire;
use crate::data::instr::issue::{
    ImportChapterIssuesInstr, IssueInstr, IssueRectInstr, PageIssuesInstr,
};
use crate::data::val::issue::ImportChapterIssuesVal;
use crate::data::view::issue::{IssueInfoView, IssueRectView};

/// A finite rectangle contained within the normalized whole composite.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueRectBody {
    /// Left edge, in [0, 1].
    pub x_coord: f64,
    /// Top edge, in [0, 1].
    pub y_coord: f64,
    /// Positive width, with the right edge at most 1.
    pub width: f64,
    /// Positive height, with the bottom edge at most 1.
    pub height: f64,
}

impl From<IssueRectBody> for IssueRectInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueRectBody) -> Self {
        Self {
            x_coord: value.x_coord,
            y_coord: value.y_coord,
            width: value.width,
            height: value.height,
        }
    }
}

/// One issue in a complete chapter import.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueBody {
    /// Open category, required to contain non-whitespace text.
    pub variant: String,
    /// Optional normalized whole-page rectangle.
    pub rect: Option<IssueRectBody>,

    /// Original text, preserved verbatim including empty strings and newlines.
    pub note: String,
}

impl From<IssueBody> for IssueInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueBody) -> Self {
        Self {
            variant: value.variant,
            rect: value.rect.map(std::convert::Into::into),
            note: value.note,
        }
    }
}

/// Issues for one explicit composite page in the imported review file.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageIssuesBody {
    /// Stable composite page ID within the target Chapter.
    pub page_artwork_id: String,
    /// Array order determines the zero-based issue index.
    pub issues: Vec<IssueBody>,
}

impl From<PageIssuesBody> for PageIssuesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageIssuesBody) -> Self {
        Self {
            page_artwork_id: value.page_artwork_id,
            issues: value
                .issues
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Replaces the single current review for a Chapter using explicit composite IDs.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterIssuesBody {
    /// Composite page IDs with their issues; omitted pages have no issues.
    pub pages: Vec<PageIssuesBody>,
}

impl From<ImportChapterIssuesBody> for ImportChapterIssuesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ImportChapterIssuesBody) -> Self {
        Self {
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Counts for the complete chapter import, including empty review pages.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterIssuesPayload {
    /// Number of pages in the imported review file.
    pub imported_page_count: usize,
    /// Number of newly inserted issues.
    pub imported_issue_count: usize,
}

impl From<ImportChapterIssuesVal> for ImportChapterIssuesPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ImportChapterIssuesVal) -> Self {
        Self {
            imported_page_count: value.imported_page_count,
            imported_issue_count: value.imported_issue_count,
        }
    }
}

impl Wire for ImportChapterIssuesVal {
    // Serializable wire payload produced by this conversion.
    type Target = ImportChapterIssuesPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Geometry in normalized whole-page coordinates.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueRectObj {
    /// Left edge.
    pub x_coord: f64,
    /// Top edge.
    pub y_coord: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl From<IssueRectView> for IssueRectObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueRectView) -> Self {
        Self {
            x_coord: value.x_coord,
            y_coord: value.y_coord,
            width: value.width,
            height: value.height,
        }
    }
}

impl Wire for IssueRectView {
    // Serializable wire payload produced by this conversion.
    type Target = IssueRectObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Read-only issue in the chapter's single current review.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueInfoObj {
    /// Server-generated identity.
    pub id: String,

    /// Owning composite page identity.
    pub page_artwork_id: String,
    /// Zero-based position within the review page.
    pub index: usize,

    /// Open category.
    pub variant: String,
    /// Optional whole-page rectangle.
    pub rect: Option<IssueRectObj>,

    /// Original note text.
    pub note: String,
}

impl From<IssueInfoView> for IssueInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: IssueInfoView) -> Self {
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

impl Wire for IssueInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = IssueInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
