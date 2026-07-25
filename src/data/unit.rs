//! Data transfer objects for page unit use cases.

use serde::{Deserialize, Serialize};

use poprako_util::time::ToUnixMilli;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::unit::{UnitContent, UnitCounters, UnitDiff, UnitIdMapper, UnitInfo, UnitOper};

#[cfg(test)]
mod tests;

/// Presentation-ready unit information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitInfoVal {
    //
    /// Unique unit identifier.
    pub id: String,

    /// Parent page identifier this unit belongs to.
    pub page_id: String,

    /// Whether this unit represents a speech bubble area on the page.
    pub is_bubble: bool,
    /// Whether this unit has an approved proofread version.
    pub is_proofread: bool,

    /// Horizontal coordinate of the unit bounding box on the page.
    pub x_coord: f64,
    /// Vertical coordinate of the unit bounding box on the page.
    pub y_coord: f64,

    /// Translated text content, or [`None`] if not yet translated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// Identifier of the user who last modified the translation, or [`None`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_translator_id: Option<String>,

    /// Proofread text content, or [`None`] if not yet proofread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// Identifier of the user who last modified the proofread, or [`None`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_proofreader_id: Option<String>,

    /// Timestamp of unit creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last unit update, in milliseconds since Unix epoch.
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

/// Input parameters for listing all units under one page.
#[derive(Debug, Deserialize)]
pub struct ListPageUnitInfosParams {
    /// Parent page identifier to list units for.
    pub page_id: String,
}

/// Return value for listing units under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListPageUnitInfosPayload {
    //
    /// All units belonging to the requested page.
    pub unit_infos: Vec<UnitInfoVal>,

    /// Total number of units on the page.
    pub total_unit_count: i32,
    /// Number of units that have a translation.
    pub translated_unit_count: i32,
    /// Number of units that have been proofread.
    pub proofread_unit_count: i32,
}

/// Input parameters for saving unit opers under one page.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageUnitsParams {
    //
    /// Parent page identifier to save units under.
    pub page_id: String,
    /// Batch of unit operations to apply (create, save, delete).
    pub diff: UnitDiffParams,
}

/// Return value for saving unit opers under one page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SavePageUnitsPayload {
    //
    /// Mappings from client-assigned local IDs to server-assigned unit IDs.
    pub local_id_mappers: Vec<UnitIdMapperVal>,

    /// Total number of units on the page after the save.
    pub total_unit_count: i32,
    /// Number of translated units after the save.
    pub translated_unit_count: i32,
    /// Number of proofread units after the save.
    pub proofread_unit_count: i32,
}

/// Transport-facing unit oper.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitDiffParams {
    //
    /// Parent page identifier the operations apply to.
    pub page_id: String,
    /// Ordered list of unit operations to apply.
    pub opers: Vec<UnitOperParams>,
}

/// Transport-facing unit oper event.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "oper", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitOperParams {
    /// Create a new unit with client-assigned local id and content payload.
    Create {
        //
        /// Client-assigned temporary identifier mapped to the server-assigned id after creation.
        local_id: String,

        /// Identifier of the unit to insert before in ordering, or [`None`] to append.
        #[serde(default)]
        before_id: Option<String>,

        /// Whether this unit represents a speech bubble area.
        is_bubble: bool,

        /// Whether this unit should be marked as proofread immediately.
        #[serde(default)]
        is_proofread: bool,

        /// Horizontal position of the unit on the page.
        x_coord: f64,
        /// Vertical position of the unit on the page.
        y_coord: f64,

        /// Initial translated text content, or [`None`].
        translated_text: Option<String>,
        /// Identifier of the user providing the initial translation, or [`None`].
        last_translator_id: Option<String>,

        /// Initial proofread text content, or [`None`].
        proofread_text: Option<String>,
        /// Identifier of the user providing the initial proofread, or [`None`].
        last_proofreader_id: Option<String>,
    },

    /// Update an existing unit identified by server-assigned id with new content payload.
    Save {
        //
        /// Server-assigned identifier of the unit to update.
        id: String,

        /// Identifier of the unit to insert before in ordering, or [`None`] to keep current position.
        #[serde(default)]
        before_id: Option<String>,

        /// Whether this unit represents a speech bubble area.
        is_bubble: bool,
        /// Whether this unit has an approved proofread version.
        is_proofread: bool,

        /// Updated horizontal position of the unit on the page.
        x_coord: f64,
        /// Updated vertical position of the unit on the page.
        y_coord: f64,

        /// Updated translated text content, or [`None`] to leave unchanged.
        translated_text: Option<String>,
        /// Identifier of the user providing the updated translation, or [`None`].
        last_translator_id: Option<String>,

        /// Updated proofread text content, or [`None`] to leave unchanged.
        proofread_text: Option<String>,
        /// Identifier of the user providing the updated proofread, or [`None`].
        last_proofreader_id: Option<String>,
    },

    /// Remove an existing unit by server-assigned id.
    Delete {
        /// Server-assigned identifier of the unit to remove.
        id: String,
    },
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
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitIdMapperVal {
    //
    /// Client-assigned temporary unit identifier.
    pub local_id: String,
    /// Server-assigned permanent unit identifier.
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
