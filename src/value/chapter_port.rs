use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

/// Translation format used by a chapter port.
///
/// Determines the tooling and schema for the chapter's translation files.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema,
)]
pub enum TranslationFormat {
    /// LabelPlus translation format.
    #[serde(rename = "label-plus")]
    LabelPlus,
    /// PopRaKo native translation format.
    #[serde(rename = "poprako")]
    PopRaKo,
}
