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
use crate::model::shared::unit_save::UnitSaveCreatedUnitId;

/// Return value for listing visible Units under one Page.
#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListPageUnitInfosVal {
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
        count_metrics: UnitCountMetrics,
    ) -> Self {
        //
        Self {
            unit_infos: unit_infos
                .into_iter()
                .filter(|unit_info| unit_info.hidden_at.is_none())
                .map(UnitInfoView::from)
                .collect(),
            total_unit_count: count_metrics.total,
            translated_unit_count: count_metrics.translated,
            proofread_unit_count: count_metrics.proofread,
        }
    }
}

/// One explicit identity pair returned for a newly created Unit.
#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreatedUnitIdVal {
    /// Client identity from the create operation.
    pub local_id: String,
    /// Permanent Unit identity.
    pub unit_id: String,
}

/// Result of every successful Unit save, including replayed saves.
#[derive(Serialize)]
#[cfg_attr(test, derive(Debug))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageUnitEditsVal {
    /// Explicit identity pairs; empty when the batch creates no Units.
    pub created_unit_ids: Vec<CreatedUnitIdVal>,
}

impl From<Vec<UnitSaveCreatedUnitId>> for SavePageUnitEditsVal {
    // Convert shared identities into the transport response shape.
    fn from(ids: Vec<UnitSaveCreatedUnitId>) -> Self {
        //
        Self {
            created_unit_ids: ids
                .into_iter()
                .map(|pair| CreatedUnitIdVal {
                    local_id: pair.local_id,
                    unit_id: pair.unit_id,
                })
                .collect(),
        }
    }
}
