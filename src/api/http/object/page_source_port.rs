//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::unit_port::UnitTranslationPortObj;
use crate::api::http::result::Wire;
use crate::data::view::page_source_port::PageSourceTranslationPortView;

/// source page object exchanged by the `PopRaKo` translation port.
#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceTranslationPortObj {
    /// source page identifier from the exporting chapter.
    pub page_id: String,
    /// Zero-based page index within its chapter.
    pub page_index: usize,

    /// Units belonging to this page.
    pub units: Vec<UnitTranslationPortObj>,
}

impl From<PageSourceTranslationPortView> for PageSourceTranslationPortObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageSourceTranslationPortView) -> Self {
        Self {
            page_id: value.page_id,
            page_index: value.page_index,
            units: value
                .units
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for PageSourceTranslationPortView {
    // Serializable wire payload produced by this conversion.
    type Target = PageSourceTranslationPortObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
