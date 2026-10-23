//! View DTOs for page translation port import and export.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::unit_port::UnitTranslationPortView;
use crate::model::shared::translation_document::PageTranslationDocument;

/// Page object exchanged by the `PopRaKo` translation port.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageTranslationPortView {
    /// Page identifier from the exporting chapter.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: usize,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationPortView>,
}

impl From<PageTranslationDocument> for PageTranslationPortView {
    // Converts the native exchange document into its response representation.
    fn from(document: PageTranslationDocument) -> Self {
        Self {
            page_id: document.page_id,
            page_index: document.page_index,
            units: document.units.into_iter().map(Into::into).collect(),
        }
    }
}
