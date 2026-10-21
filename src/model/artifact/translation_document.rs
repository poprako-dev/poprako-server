//! Native translation exchange documents independent of response DTOs.

use serde::Deserialize;

/// Unit object exchanged by the `PopRaKo` translation port.
#[derive(Deserialize)]
pub struct UnitTranslationDocument {
    /// Unit identifier from the exporting chapter.
    pub unit_id: String,

    /// Ordinal index of the unit within its page.
    pub unit_index: usize,

    /// Parent page identifier from the exporting chapter.
    pub page_id: String,

    /// Ordinal index of the page within its chapter.
    pub page_index: usize,

    /// Horizontal coordinate of the unit bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit bounding box on the page.
    pub y_coord: f64,

    /// Whether this unit represents a speech bubble area.
    pub is_bubble: bool,
    /// Whether this Unit is flagged for later review.
    #[serde(default)]
    pub is_flagged: bool,

    /// Translated text content, or [`None`] if not translated.
    pub translated_text: Option<String>,
    /// Identifier of the translator user, or [`None`]. This is export metadata.
    pub translator_id: Option<String>,

    /// Whether this unit has been proofread.
    pub is_proofread: bool,
    /// Proofread text content, or [`None`] if not proofread.
    pub proofread_text: Option<String>,
    /// Identifier of the proofreader user, or [`None`]. This is export metadata.
    pub proofreader_id: Option<String>,
}

/// Page object exchanged by the `PopRaKo` translation port.
#[derive(Deserialize)]
pub struct PageTranslationDocument {
    /// Page identifier from the exporting chapter.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: usize,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationDocument>,
}

/// JSON document exchanged by the `PopRaKo` translation port.
#[derive(Deserialize)]
pub struct ChapterTranslationDocument {
    /// Chapter identifier from the exporting chapter.
    pub chapter_id: String,
    /// Ordinal index of the chapter within its comic.
    pub chapter_index: usize,
    /// Optional subtitle from the exporting chapter.
    pub chapter_subtitle: Option<String>,

    /// Comic identifier from the exporting comic.
    pub comic_id: String,
    /// Comic title from the exporting comic.
    pub comic_title: String,

    /// Pages and their translation units.
    pub pages: Vec<PageTranslationDocument>,
}
