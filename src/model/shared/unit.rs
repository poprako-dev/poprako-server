#[derive(Clone)]
pub struct UnitCoord {
    pub x_coord: f64,
    pub y_coord: f64,
}

pub struct UnitTranslation {
    /// Translated text content, absent when not yet translated.
    pub translated_text: String,
    /// User who last modified the translated text.
    pub last_translator_id: String,
}

pub struct UnitRevision {
    /// Proofread (reviewed) text content, absent when not yet proofread.
    pub proofread_text: String,
    /// User who last modified the proofread text.
    pub last_proofreader_id: String,
}
