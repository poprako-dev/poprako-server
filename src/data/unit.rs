//! Data transfer objects for page unit use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::unit::{
    UnitCounters, UnitIdMapper, UnitInfo, UnitLocalSnapshot, UnitOper, UnitServerSnapshot,
};

/// Presentation-ready unit information.
pub struct UnitInfoVal {
    pub id: String,

    pub page_id: String,
    pub index: i32,

    pub is_bubble: bool,
    pub is_proofread: bool,

    pub x_coord: f64,
    pub y_coord: f64,

    pub translated_text: Option<String>,
    pub translator_comment: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub proofreader_comment: Option<String>,
    pub last_proofreader_id: Option<String>,

    pub created_at: i64,
    pub updated_at: i64,
}

impl From<UnitInfo> for UnitInfoVal {
    fn from(model: UnitInfo) -> Self {
        Self {
            id: model.id,
            page_id: model.page_id,
            index: model.index,
            is_bubble: model.is_bubble,
            is_proofread: model.is_proofread,
            x_coord: model.x_coord,
            y_coord: model.y_coord,
            translated_text: model.translated_text,
            translator_comment: model.translator_comment,
            last_translator_id: model.last_translator_id,
            proofread_text: model.proofread_text,
            proofreader_comment: model.proofreader_comment,
            last_proofreader_id: model.last_proofreader_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for listing units under one page.
pub struct ListPageUnitInfosData {
    pub page_id: String,
}

/// Return value for listing units under one page.
pub struct ListPageUnitInfosVal {
    pub units: Vec<UnitInfoVal>,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit operations under one page.
pub struct SavePageUnitsData {
    pub page_id: String,
    pub opers: Vec<UnitOperationData>,
}

/// Return value for saving unit operations under one page.
pub struct SavePageUnitsVal {
    pub units: Vec<UnitInfoVal>,
    pub local_id_mappings: Vec<UnitLocalIdMappingVal>,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Transport-facing unit operation.
pub enum UnitOperationData {
    Update {
        unit: UnitServerSnapshot,
    },
    MoveBefore {
        unit: UnitServerSnapshot,
        before_id: Option<String>,
    },
    InsertBefore {
        unit: UnitLocalSnapshot,
        before_id: Option<String>,
    },
    Delete {
        unit_id: String,
    },
}

impl From<UnitOperationData> for UnitOper {
    fn from(value: UnitOperationData) -> Self {
        match value {
            UnitOperationData::Update { unit } => Self::Update { unit },
            UnitOperationData::MoveBefore { unit, before_id } => {
                Self::MoveBefore { unit, before_id }
            }
            UnitOperationData::InsertBefore { unit, before_id } => {
                Self::InsertBefore { unit, before_id }
            }
            UnitOperationData::Delete { unit_id } => Self::Delete { unit_id },
        }
    }
}

/// Presentation-ready local-to-server id mapping.
pub struct UnitLocalIdMappingVal {
    pub local_id: String,
    pub unit_id: String,
}

impl From<UnitIdMapper> for UnitLocalIdMappingVal {
    fn from(model: UnitIdMapper) -> Self {
        Self {
            local_id: model.local_id,
            unit_id: model.unit_id,
        }
    }
}

impl SavePageUnitsVal {
    /// Builds a save response from final units, mappings, and counters.
    pub fn from_parts(
        units: Vec<UnitInfo>,
        local_id_mappings: Vec<UnitIdMapper>,
        counters: UnitCounters,
    ) -> Self {
        Self {
            units: units.into_iter().map(UnitInfoVal::from).collect(),
            local_id_mappings: local_id_mappings
                .into_iter()
                .map(UnitLocalIdMappingVal::from)
                .collect(),
            total_unit_count: counters.total_unit_count,
            translated_unit_count: counters.translated_unit_count,
            proofread_unit_count: counters.proofread_unit_count,
        }
    }
}
