//! View DTOs for page translation port import and export.

use serde::{Deserialize, Serialize};
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::unit_port::UnitTranslationPortView;

/// Page object exchanged by the PopRaKo translation port.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageTranslationPortView {
    /// Page identifier from the exporting chapter.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: i32,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationPortView>,
}
