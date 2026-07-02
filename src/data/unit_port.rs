//! Data transfer objects for unit import/export port use cases.

/// JSON-safe export object for one page unit.
pub struct UnitTranslationExportVal {
    pub unit_id: String,
    pub unit_index: i32,

    pub page_id: String,
    pub page_index: i32,

    pub x_coord: f64,
    pub y_coord: f64,

    pub is_bubble: bool,

    pub translated_text: Option<String>,
    pub translator_id: Option<String>,

    pub is_proofread: bool,

    pub proofread_text: Option<String>,
    pub proofreader_id: Option<String>,
}
