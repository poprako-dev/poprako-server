//! Data transfer objects for page unit use cases.

use serde::{Deserialize, Serialize};

use poprako_util::time::ToUnixMilli;
#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::model::unit::{
    UnitContent, UnitCounters, UnitDiff, UnitIdMapper, UnitInfo, UnitOper,
};

#[cfg(test)]
mod tests;

/// Presentation-ready unit information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitInfoVal {
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

impl From<UnitInfo> for UnitInfoVal {
    fn from(model: UnitInfo) -> Self {
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
pub struct ListPageUnitInfosParams {
    pub page_id: String,

    pub offset: u32,
    pub limit: u32,
}

/// Return value for listing units under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ListPageUnitInfosPayload {
    pub unit_infos: Vec<UnitInfoVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit opers under one page.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageUnitsParams {
    pub page_id: String,
    pub diff: UnitDiffParams,
}

/// Return value for saving unit opers under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageUnitsPayload {
    pub local_id_mappers: Vec<UnitIdMapperVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Transport-facing unit oper.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitDiffParams {
    pub page_id: String,
    pub opers: Vec<UnitOperParams>,
}

/// Transport-facing unit oper event.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(tag = "oper", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitOperParams {
    /// Create a new unit with client-assigned local id and content payload.
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
    /// Update an existing unit identified by server-assigned id with new content payload.
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
    /// Remove an existing unit by server-assigned id.
    Delete { id: String },
}

impl UnitDiffParams {
    /// Converts transport-safe data into domain opers.
    pub fn into_model(self) -> Option<UnitDiff> {
        //
        let mut opers = Vec::with_capacity(self.opers.len());

        for unit_oper_data in self.opers {
            //
            let unit_oper = unit_oper_data.into_model();

            opers.push(unit_oper);
        }

        Some(UnitDiff {
            page_id: self.page_id,
            opers,
        })
    }
}

impl UnitOperParams {
    fn into_model(self) -> UnitOper {
        match self {
            //
            UnitOperParams::Create {
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
            } => UnitOper::Create {
                id: local_id,
                payload: UnitContent {
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
            UnitOperParams::Save {
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
            } => UnitOper::Save {
                id,
                payload: UnitContent {
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

            UnitOperParams::Delete { id } => UnitOper::Delete { id },
        }
    }
}

/// Presentation-ready local-to-server id mapping.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitIdMapperVal {
    pub local_id: String,
    pub unit_id: String,
}

impl From<UnitIdMapper> for UnitIdMapperVal {
    fn from(model: UnitIdMapper) -> Self {
        Self {
            local_id: model.local_id,
            unit_id: model.unit_id,
        }
    }
}

impl SavePageUnitsPayload {
    /// Builds a compact save response from mappings and counters.
    pub fn from_parts(
        local_id_mappers: Vec<UnitIdMapper>,
        counters: UnitCounters,
    ) -> Self {
        Self {
            local_id_mappers: local_id_mappers
                .into_iter()
                .map(UnitIdMapperVal::from)
                .collect(),
            total_unit_count: counters.total_unit_count,
            translated_unit_count: counters.translated_unit_count,
            proofread_unit_count: counters.proofread_unit_count,
        }
    }
}
