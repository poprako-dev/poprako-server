//! Instr DTOs for the chapter port domain.

//! Data transfer objects for chapter import/export port use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::chapter_port::TranslationFormat;

/// Request body for importing chapter translations.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationInstr {
    /// The translation format (e.g., JSON, SRT).
    pub format: TranslationFormat,
    /// Raw translation content string.
    pub content: String,
}
