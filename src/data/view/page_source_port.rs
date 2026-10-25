//! View DTOs for page translation port import and export.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::unit_port::UnitTranslationPortView;
use crate::model::shared::translation_document::PageSourceTranslationDocument;

/// source page object exchanged by the `PopRaKo` translation port.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceTranslationPortView {
    /// source page identifier from the exporting chapter.
    #[serde(rename = "page_id")]
    pub page_source_id: String,
    /// Zero-based page index within its chapter.
    #[serde(rename = "page_index")]
    pub page_source_index: usize,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationPortView>,
}

impl From<PageSourceTranslationDocument> for PageSourceTranslationPortView {
    // Converts the native exchange document into its response representation.
    fn from(document: PageSourceTranslationDocument) -> Self {
        Self {
            page_source_id: document.page_source_id,
            page_source_index: document.page_source_index,
            units: document.units.into_iter().map(Into::into).collect(),
        }
    }
}
