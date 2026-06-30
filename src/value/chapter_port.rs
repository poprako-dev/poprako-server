use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum TranslationFormat {
    #[serde(rename = "label-plus")]
    LabelPlus,
    #[serde(rename = "poprako")]
    PopRaKo,
}
