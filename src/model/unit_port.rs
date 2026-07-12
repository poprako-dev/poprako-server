//! Internal normalized unit payloads for chapter translation import.

use serde::Deserialize;

/// One parsed import unit.
pub struct TranslationImport {
    pub id: Option<String>,
    pub index: i32,
    pub x_coord: f64,
    pub y_coord: f64,
    pub is_bubble: bool,
    pub main_text: Option<String>,

    pub translated_text: Option<String>,
    pub proofread_text: Option<String>,
    pub is_proofread: bool,
}

/// PopRaKo JSON import unit.
#[derive(Deserialize)]
pub struct PoprakoImport {
    pub id: String,

    pub x: f64,
    pub y: f64,
    pub index_in_page: i32,
    pub is_inbox: bool,

    pub translated_text: Option<String>,
    pub prooved_text: Option<String>,
    pub is_prooved: bool,
}
