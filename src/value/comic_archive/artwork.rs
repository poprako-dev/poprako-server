//! Immutable composite page archive payloads.

use serde::Serialize;

/// One independent composite page and its current review.
#[derive(Serialize)]
pub struct ArchivedPageArtworkPayload<'a> {
    /// Original composite page identity.
    pub source_page_artwork_id: &'a str,

    /// Position within the composite sequence.
    pub index: usize,

    /// Original PSD filename or relative path.
    pub raw_ident: Option<&'a str>,
    /// Ordered current review details.
    pub issues: Vec<ArchivedIssuePayload<'a>>,

    /// Creation time in Unix milliseconds.
    pub created_at: i64,
    /// Last metadata update in Unix milliseconds.
    pub updated_at: i64,
}

/// Whole-page rectangle retained in an immutable archive.
#[derive(Serialize)]
pub struct ArchivedIssueRectPayload {
    /// Left edge.
    pub x_coord: f64,
    /// Top edge.
    pub y_coord: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// Read-only issue retained in an immutable archive.
#[derive(Serialize)]
pub struct ArchivedIssuePayload<'a> {
    /// Original issue identity.
    pub id: &'a str,

    /// Owning composite page identity.
    pub page_artwork_id: &'a str,
    /// Zero-based position within the composite page.
    pub index: usize,

    /// Open category.
    pub variant: &'a str,
    /// Optional human-readable layer name, preserved verbatim.
    pub layer_name: Option<&'a str>,
    /// Optional whole-page rectangle.
    pub rect: Option<ArchivedIssueRectPayload>,

    /// Original note text.
    pub note: &'a str,
}
