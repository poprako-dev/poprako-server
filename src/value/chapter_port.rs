use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Maximum number of pages accepted in one chapter import.
pub const MAX_CHAPTER_IMPORT_PAGE_COUNT: usize = 200;

/// Translation format used by a chapter port.
///
/// Determines the tooling and schema for the chapter's translation files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub enum TranslationFormat {
    /// `LabelPlus` translation format.
    #[serde(rename = "label-plus")]
    LabelPlus,

    /// `PopRaKo` native translation format.
    #[serde(rename = "poprako")]
    PopRaKo,
}

/// Formats generated together by one chapter translation export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportFormatSpec {
    /// Whether to generate `LabelPlus` text.
    label_plus: bool,
    /// Whether to generate the native `PopRaKo` document.
    poprako: bool,
}

impl ExportFormatSpec {
    /// Selects only `LabelPlus` output.
    pub const LABEL_PLUS: Self = Self {
        label_plus: true,
        poprako: false,
    };

    /// Selects only native `PopRaKo` output.
    pub const POPRAKO: Self = Self {
        label_plus: false,
        poprako: true,
    };

    /// Selects both supported output formats.
    pub const BOTH: Self = Self {
        label_plus: true,
        poprako: true,
    };

    /// Returns whether `LabelPlus` output is selected.
    pub const fn includes_label_plus(self) -> bool {
        self.label_plus
    }

    /// Returns whether native `PopRaKo` output is selected.
    pub const fn includes_poprako(self) -> bool {
        self.poprako
    }
}

impl<'de> Deserialize<'de> for ExportFormatSpec {
    // Deserialize the selected export formats and reject an empty selection.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Capture the serialized format flags before validating their combination.
        #[derive(Deserialize)]
        struct Fields {
            // Whether to generate LabelPlus output.
            label_plus: bool,

            // Whether to generate native PopRaKo output.
            poprako: bool,
        }

        let fields = Fields::deserialize(deserializer)?;

        match (fields.label_plus, fields.poprako) {
            //
            (true, false) => Ok(Self::LABEL_PLUS),

            (false, true) => Ok(Self::POPRAKO),

            (true, true) => Ok(Self::BOTH),

            (false, false) => {
                Err(D::Error::custom("at least one export format is required"))
            }
        }
    }
}

impl From<TranslationFormat> for ExportFormatSpec {
    // Convert one translation format into its corresponding export selection.
    fn from(format: TranslationFormat) -> Self {
        //
        match format {
            //
            TranslationFormat::LabelPlus => Self::LABEL_PLUS,

            TranslationFormat::PopRaKo => Self::POPRAKO,
        }
    }
}
