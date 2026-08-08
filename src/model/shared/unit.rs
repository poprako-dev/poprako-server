//! Value groups shared by Unit read and write models.

/// Page-relative Unit coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitCoord {
    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,
}

/// Translation content together with its server-derived editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitTranslation {
    /// Current translated text.
    pub translated_text: String,
    /// ID of the translator assigning this content.
    pub last_translator_id: String,
}

/// Revision content and approval state together with its server-derived editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRevision {
    /// Whether the revision is approved.
    pub is_proofread: bool,
    /// Current proofread text.
    pub proofread_text: Option<String>,
    /// ID of the proofreader assigning this revision.
    pub last_proofreader_id: String,
}
