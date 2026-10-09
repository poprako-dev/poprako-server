//! Chapter review import instructions, independent of read projections.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::shared::issue::IssueRect;

/// A finite rectangle contained within the normalized whole Page.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueRectInstr {
    /// Left edge, in [0, 1].
    pub x_coord: f64,
    /// Top edge, in [0, 1].
    pub y_coord: f64,
    /// Positive width, with the right edge at most 1.
    pub width: f64,
    /// Positive height, with the bottom edge at most 1.
    pub height: f64,
}

impl From<IssueRectInstr> for IssueRect {
    // Converts the domain value into its corresponding representation.
    fn from(instr: IssueRectInstr) -> Self {
        Self {
            x_coord: instr.x_coord,
            y_coord: instr.y_coord,
            width: instr.width,
            height: instr.height,
        }
    }
}

/// One issue in a complete chapter import.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueInstr {
    /// Open category, required to contain non-whitespace text.
    pub variant: String,
    /// Optional opaque nonblank layer path.
    pub layer_path: Option<String>,
    /// Optional normalized whole-page rectangle, independent of layer selection.
    pub rect: Option<IssueRectInstr>,

    /// Original text, preserved verbatim including empty strings and newlines.
    pub note: String,
}

/// Issues for one position in the chapter's current Page manifest.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageIssuesInstr {
    /// Array order determines the zero-based issue index.
    pub issues: Vec<IssueInstr>,
}

/// Replaces the single current review for a Chapter in current Page order.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterIssuesInstr {
    /// Exactly one item per current Page, including Pages without issues.
    pub pages: Vec<PageIssuesInstr>,
}
