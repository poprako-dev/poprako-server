//! Data transfer objects for page unit use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::model::unit_model;

/// Presentation-ready unit information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct InfoVal {
    pub id: String,

    pub page_id: String,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_translator_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_proofreader_id: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<unit_model::Info> for InfoVal {
    fn from(model: unit_model::Info) -> Self {
        Self {
            id: model.id,
            page_id: model.page_id,
            is_bubble: model.is_bubble,
            is_proofread: model.is_proofread,
            x_coord: model.x_coord,
            y_coord: model.y_coord,
            translated_text: model.translated_text,
            last_translator_id: model.last_translator_id,
            proofread_text: model.proofread_text,
            last_proofreader_id: model.last_proofreader_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for listing units under one page.
#[derive(Debug, Deserialize)]
pub struct ListPageInfosData {
    pub page_id: String,

    pub offset: u32,
    pub limit: u32,
}

/// Return value for listing units under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ListPageInfosVal {
    pub unit_infos: Vec<InfoVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit opers under one page.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageData {
    pub page_id: String,
    pub diff: DiffData,
}

/// Return value for saving unit opers under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageVal {
    pub local_id_mappers: Vec<IdMapperVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Transport-facing unit oper.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct DiffData {
    pub page_id: String,
    pub opers: Vec<OperData>,
}

/// Transport-facing unit oper event.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(tag = "oper", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperData {
    Create {
        local_id: String,

        #[serde(default)]
        before_id: Option<String>,

        is_bubble: bool,

        #[serde(default)]
        is_proofread: bool,

        x_coord: f64,
        y_coord: f64,

        translated_text: Option<String>,
        last_translator_id: Option<String>,

        proofread_text: Option<String>,
        last_proofreader_id: Option<String>,
    },
    Save {
        id: String,

        #[serde(default)]
        before_id: Option<String>,

        is_bubble: bool,
        is_proofread: bool,

        x_coord: f64,
        y_coord: f64,

        translated_text: Option<String>,
        last_translator_id: Option<String>,

        proofread_text: Option<String>,
        last_proofreader_id: Option<String>,
    },
    Delete {
        id: String,
    },
}

impl DiffData {
    /// Converts transport-safe data into domain opers.
    pub fn into_model(self) -> Option<unit_model::Diff> {
        //
        let mut opers = Vec::with_capacity(self.opers.len());

        for unit_oper_data in self.opers {
            //
            let unit_oper = unit_oper_data.into_model();

            opers.push(unit_oper);
        }

        Some(unit_model::Diff {
            page_id: self.page_id,
            opers,
        })
    }
}

impl OperData {
    fn into_model(self) -> unit_model::Oper {
        match self {
            //
            OperData::Create {
                local_id,
                before_id,
                is_bubble,
                is_proofread,
                x_coord,
                y_coord,
                translated_text,
                last_translator_id,
                proofread_text,
                last_proofreader_id,
            } => unit_model::Oper::Create {
                id: local_id,
                payload: unit_model::Payload {
                    is_bubble,
                    is_proofread,
                    x_coord,
                    y_coord,
                    translated_text,
                    last_translator_id,
                    proofread_text,
                    last_proofreader_id,
                },
                before_id,
            },

            //
            OperData::Save {
                id,
                before_id,
                is_bubble,
                is_proofread,
                x_coord,
                y_coord,
                translated_text,
                last_translator_id,
                proofread_text,
                last_proofreader_id,
            } => unit_model::Oper::Save {
                id,
                payload: unit_model::Payload {
                    is_bubble,
                    is_proofread,
                    x_coord,
                    y_coord,
                    translated_text,
                    last_translator_id,
                    proofread_text,
                    last_proofreader_id,
                },
                before_id,
            },

            OperData::Delete { id } => unit_model::Oper::Delete { id },
        }
    }
}

/// Presentation-ready local-to-server id mapping.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct IdMapperVal {
    pub local_id: String,
    pub unit_id: String,
}

impl From<unit_model::IdMapper> for IdMapperVal {
    fn from(model: unit_model::IdMapper) -> Self {
        Self {
            local_id: model.local_id,
            unit_id: model.unit_id,
        }
    }
}

impl SavePageVal {
    /// Builds a compact save response from mappings and counters.
    pub fn from_parts(
        local_id_mappers: Vec<unit_model::IdMapper>,
        counters: unit_model::Counters,
    ) -> Self {
        Self {
            local_id_mappers: local_id_mappers
                .into_iter()
                .map(IdMapperVal::from)
                .collect(),
            total_unit_count: counters.total_unit_count,
            translated_unit_count: counters.translated_unit_count,
            proofread_unit_count: counters.proofread_unit_count,
        }
    }
}

#[cfg(test)]
mod tests;
