use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum TranslationFormat {
    #[serde(rename = "label-plus")]
    LabelPlus,
    #[serde(rename = "poprako")]
    PopRaKo,
}
