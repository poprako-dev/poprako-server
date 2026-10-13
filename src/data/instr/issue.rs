//! Chapter review import instructions, independent of read projections.

#[cfg(test)]
mod tests;

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::shared::issue::IssueRect;

/// A finite rectangle contained within the normalized whole composite.
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
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct IssueInstr {
    /// Open category, required to contain non-whitespace text.
    pub variant: String,
    /// Optional human-readable, nonblank layer name, preserved verbatim.
    pub layer_name: Option<String>,
    /// Optional normalized whole-page rectangle, independent of layer selection.
    pub rect: Option<IssueRectInstr>,

    /// Original text, preserved verbatim including empty strings and newlines.
    pub note: String,
}

/// Issues for one explicit composite page in the imported review file.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageIssuesInstr {
    /// Stable composite page ID within the target Chapter.
    pub page_artwork_id: String,
    /// Array order determines the zero-based issue index.
    pub issues: Vec<IssueInstr>,
}

/// Replaces the single current review for a Chapter using explicit composite IDs.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterIssuesInstr {
    /// Composite page IDs with their issues; omitted pages have no issues.
    pub pages: Vec<PageIssuesInstr>,
}
