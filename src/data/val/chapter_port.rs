//! Val DTOs for the chapter port domain.

//! Data transfer objects for chapter import/export port use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Summary returned after importing chapter translations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationVal {
    /// Number of pages that were imported.
    pub imported_page_count: i32,
    /// Number of translation units that were imported.
    pub imported_unit_count: i32,
}
