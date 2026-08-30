//! Val DTOs for the unit domain.

//! Data transfer objects for page Unit use cases.
//!
//! Types in this module describe how client-supplied edit payloads are
//! represented and how persisted Unit rows are projected back into API-facing
//! response types.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::unit::UnitInfoView;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo};

/// Return value for listing visible Units under one Page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListPageUnitInfosVal {
    //
    /// Visible Units in final linked-list order.
    pub unit_infos: Vec<UnitInfoView>,

    /// Number of visible Units.
    pub total_unit_count: usize,
    /// Number of visible translated Units.
    pub translated_unit_count: usize,
    /// Number of visible proofread Units.
    pub proofread_unit_count: usize,
}

impl ListPageUnitInfosVal {
    /// Converts ordered persisted Units and counters into the response payload.
    pub fn from_parts(
        unit_infos: Vec<UnitInfo>,
        counters: UnitCountMetrics,
    ) -> Self {
        //
        Self {
            unit_infos: unit_infos
                .into_iter()
                .filter(|unit_info| unit_info.hidden_at.is_none())
                .map(UnitInfoView::from)
                .collect(),
            total_unit_count: counters.total,
            translated_unit_count: counters.translated,
            proofread_unit_count: counters.proofread,
        }
    }
}
