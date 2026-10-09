//! Geometry shared by review issue projections and replacements.

/// A rectangle in normalized whole-page coordinates, independent of layers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IssueRect {
    /// Horizontal position of the top-left corner.
    pub x_coord: f64,
    /// Vertical position of the top-left corner.
    pub y_coord: f64,
    /// Positive page-relative width.
    pub width: f64,
    /// Positive page-relative height.
    pub height: f64,
}
