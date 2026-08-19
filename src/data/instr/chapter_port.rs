//! Instr DTOs for the chapter port domain.

//! Data transfer objects for chapter import/export port use cases.

#[cfg(test)]
mod tests;

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::chapter_port::TranslationFormat;

/// Translation format accepted by the import JSON body.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterTranslationFormatInstr {
    /// LabelPlus translation format.
    LabelPlus,

    /// PopRaKo native translation format.
    #[serde(rename = "poprako")]
    PopRaKo,
}

impl From<ChapterTranslationFormatInstr> for TranslationFormat {
    // Converts the transport format into the domain value.
    fn from(format: ChapterTranslationFormatInstr) -> Self {
        //
        match format {
            //
            ChapterTranslationFormatInstr::LabelPlus => Self::LabelPlus,

            ChapterTranslationFormatInstr::PopRaKo => Self::PopRaKo,
        }
    }
}

impl From<TranslationFormat> for ChapterTranslationFormatInstr {
    // Converts the domain value into the transport format.
    fn from(format: TranslationFormat) -> Self {
        //
        match format {
            //
            TranslationFormat::LabelPlus => Self::LabelPlus,

            TranslationFormat::PopRaKo => Self::PopRaKo,
        }
    }
}

/// Request body for importing chapter translations.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportChapterTranslationInstr {
    /// The translation format (e.g., JSON, SRT).
    pub format: ChapterTranslationFormatInstr,
    /// Raw translation content string.
    pub content: String,
}
