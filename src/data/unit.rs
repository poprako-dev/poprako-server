//! Data transfer objects for page unit use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::unit::{UnitCounters, UnitDiff, UnitIdMapper, UnitInfo, UnitOper, UnitPayload};

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
    pub unit_infos: Vec<UnitInfoVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit opers under one page.
pub struct SavePageUnitsData {
    pub page_id: String,
    pub diff: UnitDiffData,
}

/// Return value for saving unit opers under one page.
pub struct SavePageUnitsVal {
    pub local_id_mappers: Vec<UnitIdMapperVal>,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

/// Transport-facing unit oper.
pub struct UnitDiffData {
    pub page_id: String,

    pub opers: Vec<UnitOperData>,

    pub candidate_order: Vec<String>,
}

/// Transport-facing compact unit oper.
pub struct UnitOperData {
    pub id: Option<String>,
    pub local_id: Option<String>,

    pub is_bubble: Option<bool>,
    pub is_proofread: Option<bool>,

    pub x_coord: Option<f64>,
    pub y_coord: Option<f64>,

    pub translated_text: Option<String>,
    pub translator_comment: Option<String>,
    pub last_translator_id: Option<String>,

    pub proofread_text: Option<String>,
    pub proofreader_comment: Option<String>,
    pub last_proofreader_id: Option<String>,
}

impl UnitDiffData {
    /// Converts transport-safe data into domain opers.
    pub fn into_model(self) -> Option<UnitDiff> {
        let mut opers = Vec::with_capacity(self.opers.len());

        for unit_oper_data in self.opers {
            let unit_oper = unit_oper_data.into_model()?;

            opers.push(unit_oper);
        }

        Some(UnitDiff {
            page_id: self.page_id,
            opers,
            candidate_order: self.candidate_order,
        })
    }
}

impl UnitOperData {
    fn into_model(self) -> Option<UnitOper> {
        let payload = self.payload();
        let id = self.id;
        let local_id = self.local_id;

        match (id, local_id, payload) {
            (None, Some(local_id), Some(payload)) => Some(UnitOper::Create {
                local_id,
                id: None,
                payload,
            }),
            (Some(id), None, Some(payload)) => Some(UnitOper::Save { id, payload }),
            (Some(id), None, None) => Some(UnitOper::Delete { id }),
            _ => None,
        }
    }

    fn payload(&self) -> Option<UnitPayload> {
        Some(UnitPayload {
            is_bubble: self.is_bubble?,
            is_proofread: self.is_proofread?,
            x_coord: self.x_coord?,
            y_coord: self.y_coord?,
            translated_text: self.translated_text.clone(),
            translator_comment: self.translator_comment.clone(),
            last_translator_id: self.last_translator_id.clone(),
            proofread_text: self.proofread_text.clone(),
            proofreader_comment: self.proofreader_comment.clone(),
            last_proofreader_id: self.last_proofreader_id.clone(),
        })
    }
}

/// Presentation-ready local-to-server id mapping.
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
    pub fn from_parts(local_id_mappers: Vec<UnitIdMapper>, counters: UnitCounters) -> Self {
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
