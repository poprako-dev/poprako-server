//! Data transfer objects for page unit use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::model::unit::{
    UnitCounters, UnitDiff, UnitIdMapper, UnitInfo, UnitOper, UnitPayload,
};

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

    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
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
#[Paginate]
#[derive(Debug, Deserialize)]
pub struct ListPageUnitInfosData {
    pub page_id: String,
}

/// Return value for listing units under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ListPageUnitInfosVal {
    pub unit_infos: Vec<UnitInfoVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit opers under one page.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageUnitsData {
    pub page_id: String,
    pub diff: UnitDiffData,
}

/// Return value for saving unit opers under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct SavePageUnitsVal {
    pub local_id_mappers: Vec<UnitIdMapperVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Transport-facing unit oper.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitDiffData {
    pub page_id: String,

    pub opers: Vec<UnitOperData>,
}

/// Transport-facing unit oper event.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(tag = "oper", rename_all = "snake_case")]
pub enum UnitOperData {
    Save {
        #[serde(default)]
        local_id: Option<String>,

        #[serde(default)]
        id: Option<String>,

        #[serde(default)]
        before_id: Option<String>,

        #[serde(flatten)]
        payload: UnitPayloadData,
    },
    Delete {
        id: String,
    },
}

/// Transport-facing complete unit payload.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UnitPayloadData {
    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub last_proofreader_id: Option<String>,
}

impl UnitDiffData {
    /// Converts transport-safe data into domain opers.
    pub fn into_model(self) -> Option<UnitDiff> {
        let mut opers = Vec::with_capacity(self.opers.len());

        for unit_oper_data in self.opers {
            let unit_oper = unit_oper_data.into_model();
            opers.push(unit_oper);
        }

        Some(UnitDiff {
            page_id: self.page_id,
            opers,
        })
    }
}

impl UnitOperData {
    fn into_model(self) -> UnitOper {
        match self {
            UnitOperData::Save {
                local_id,
                id,
                before_id,
                payload,
            } => UnitOper::Save {
                local_id,
                id,
                payload: payload.into_model(),
                before_id,
            },
            UnitOperData::Delete { id } => UnitOper::Delete { id },
        }
    }
}

impl UnitPayloadData {
    fn into_model(self) -> UnitPayload {
        UnitPayload {
            is_bubble: self.is_bubble,
            is_proofread: self.is_proofread,
            x_coord: self.x_coord,
            y_coord: self.y_coord,
            translated_text: self.translated_text,
            last_translator_id: self.last_translator_id,
            proofread_text: self.proofread_text,
            last_proofreader_id: self.last_proofreader_id,
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

impl SavePageUnitsVal {
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
