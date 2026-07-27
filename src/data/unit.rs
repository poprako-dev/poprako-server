//! Data transfer objects for page Unit use cases.
//!
//! Types in this module describe how client-supplied edit payloads are
//! represented and how persisted Unit rows are projected back into API-facing
//! response types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::read::proj::unit::{UnitCounters, UnitInfo};
use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::Patch;

#[cfg(test)]
mod tests;

/// Presentation-ready visible Unit information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UnitInfoVal {
    //
    /// Permanent Unit ID.
    pub id: String,
    /// Owning Page ID.
    pub page_id: String,

    /// Whether the Unit identifies a speech bubble.
    pub is_bubble: bool,
    /// Whether the current revision is approved.
    pub is_proofread: bool,

    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,

    /// Current translated text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_text: Option<String>,
    /// ID of the translator who last assigned translation content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_translator_id: Option<String>,

    /// Current proofread text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proofread_text: Option<String>,
    /// ID of the proofreader who last assigned revision content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_proofreader_id: Option<String>,

    /// Creation time as Unix milliseconds.
    pub created_at: i64,
    /// Last update time as Unix milliseconds.
    pub updated_at: i64,
}

impl From<UnitInfo> for UnitInfoVal {
    // Map persisted unit info model into API value shape.
    fn from(model: UnitInfo) -> Self {
        Self {
            id: model.id,
            page_id: model.page_id,
            is_bubble: model.is_bubble,
            is_proofread: model.is_proofread,
            x_coord: model.coord.x_coord,
            y_coord: model.coord.y_coord,
            translated_text: model.translated_text,
            last_translator_id: model.last_translator_id,
            proofread_text: model.proofread_text,
            last_proofreader_id: model.last_proofreader_id,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}

/// Input parameters for listing visible Units under one Page.
#[derive(Debug, Deserialize)]
pub struct ListPageUnitInfosParams {
    /// Page whose visible Units are listed.
    pub page_id: String,
}

/// Return value for listing visible Units under one Page.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ListPageUnitInfosPayload {
    //
    /// Visible Units in final linked-list order.
    pub unit_infos: Vec<UnitInfoVal>,

    /// Number of visible Units.
    pub total_unit_count: i32,
    /// Number of visible translated Units.
    pub translated_unit_count: i32,
    /// Number of visible proofread Units.
    pub proofread_unit_count: i32,
}

impl ListPageUnitInfosPayload {
    /// Converts ordered persisted Units and counters into the response payload.
    pub fn from_parts(
        unit_infos: Vec<UnitInfo>,
        counters: UnitCounters,
    ) -> Self {
        Self {
            unit_infos: unit_infos
                .into_iter()
                .filter(|unit_info| unit_info.hidden_at.is_none())
                .map(UnitInfoVal::from)
                .collect(),
            total_unit_count: counters.total_unit_count,
            translated_unit_count: counters.translated_unit_count,
            proofread_unit_count: counters.proofread_unit_count,
        }
    }
}

/// Input parameters for saving a batch of Unit edits.
#[derive(Debug, Deserialize)]
pub struct SavePageUnitEditsParams {
    //
    /// Page whose Units are being edited.
    pub page_id: String,

    /// Ordered batch of Unit edits.
    pub edits: Vec<UnitEditVal>,
}

/// One transport-facing Unit edit.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitEditVal {
    /// Creates one Unit with a request-local ID.
    Create {
        //
        /// Request-local ID used for references within this batch.
        local_id: String,

        /// Unit before which the new Unit is inserted, or the tail when null.
        #[serde(default)]
        next_id: Option<String>,

        /// Whether the Unit identifies a speech bubble.
        is_bubble: bool,
        /// Initial page-relative coordinate.
        coord: UnitCoordVal,

        /// Optional initial translation assignment.
        #[serde(default)]
        translation: Option<UnitTranslationVal>,
        /// Optional initial revision assignment.
        #[serde(default)]
        revision: Option<UnitRevisionVal>,
    },

    /// Patches or restores one permanent Unit.
    Patch {
        //
        /// Permanent target Unit ID.
        id: String,

        /// Three-state linked-list successor patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<String>)
        )]
        next_id: Patch<String>,

        /// Optional speech-bubble flag replacement.
        #[serde(default)]
        is_bubble: Option<bool>,
        /// Optional coordinate replacement.
        #[serde(default)]
        coord: Option<UnitCoordVal>,

        /// Three-state translation patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<UnitTranslationVal>)
        )]
        translation: Patch<UnitTranslationVal>,
        /// Three-state revision patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<UnitRevisionVal>)
        )]
        revision: Patch<UnitRevisionVal>,
    },

    /// Soft-deletes one permanent Unit.
    Delete {
        /// Permanent target Unit ID.
        id: String,
    },
}

/// Page-relative Unit coordinates.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitCoordVal {
    //
    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,
}

impl From<UnitCoordVal> for UnitCoord {
    // Convert API coordinate value into domain coordinate.
    fn from(value: UnitCoordVal) -> Self {
        Self {
            x_coord: value.x_coord,
            y_coord: value.y_coord,
        }
    }
}

/// Translation assignment accepted from the client.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitTranslationVal {
    /// Replacement translated text.
    pub translated_text: String,
}

/// Revision assignment accepted from the client.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitRevisionVal {
    //
    /// Replacement approval state.
    pub is_proofread: bool,

    /// Replacement proofread text.
    #[serde(default)]
    pub proofread_text: Option<String>,
}

/// Converts transport edits into domain Saves and Deletes.
///
/// The returned list preserves input order and maps request-local references.
pub fn into_unit_edits<F>(
    edits: Vec<UnitEditVal>,
    user_id: &str,
    mut gen_id: F,
) -> BaseRest<Vec<UnitEdit>>
where
    F: FnMut() -> String,
{
    let mut local_id_map = HashMap::new();

    for edit in &edits {
        //
        let UnitEditVal::Create { local_id, .. } = edit else {
            continue;
        };

        validate_id(local_id)?;

        let unit_id = gen_id();

        if local_id_map.insert(local_id.clone(), unit_id).is_some() {
            return Err(invalid_unit_edit_err());
        }
    }

    let edits = edits
        .into_iter()
        .map(|edit| edit.into_model(user_id, &local_id_map))
        .collect::<BaseRest<Vec<_>>>()?;

    accept(edits)
}

impl UnitEditVal {
    // Convert one transport edit operation into a repository edit command.
    fn into_model(
        self,
        user_id: &str,
        local_id_map: &HashMap<String, String>,
    ) -> BaseRest<UnitEdit> {
        match self {
            //
            Self::Create {
                local_id,
                next_id,
                is_bubble,
                coord,
                translation,
                revision,
            } => {
                //
                let id = resolve_id(local_id, local_id_map)?;

                let next_id = next_id
                    .map(|next_id| resolve_id(next_id, local_id_map))
                    .transpose()?;

                let translation = translation.map(|value| UnitTranslation {
                    translated_text: value.translated_text,
                    last_translator_id: user_id.to_string(),
                });

                let revision = revision.map(|value| UnitRevision {
                    is_proofread: value.is_proofread,
                    proofread_text: value.proofread_text,
                    last_proofreader_id: user_id.to_string(),
                });

                accept(UnitEdit::Create {
                    id,
                    next_id,
                    is_bubble,
                    coord: coord.into(),
                    translation,
                    revision,
                })
            }

            Self::Patch {
                id,
                next_id,
                is_bubble,
                coord,
                translation,
                revision,
            } => {
                //
                let id = resolve_id(id, local_id_map)?;

                let next_id = resolve_patch_id(next_id, local_id_map)?;

                accept(UnitEdit::Save {
                    id,
                    next_id,
                    is_bubble,
                    coord: coord.map(UnitCoord::from),
                    translation: translation.map(|value| UnitTranslation {
                        translated_text: value.translated_text,
                        last_translator_id: user_id.to_string(),
                    }),
                    revision: revision.map(|value| UnitRevision {
                        is_proofread: value.is_proofread,
                        proofread_text: value.proofread_text,
                        last_proofreader_id: user_id.to_string(),
                    }),
                })
            }

            Self::Delete { id } => accept(UnitEdit::Delete {
                id: resolve_id(id, local_id_map)?,
            }),
        }
    }
}

// Validate a unit edit local reference id is non-empty.
fn validate_id(id: &str) -> BaseRest<()> {
    //
    if id.is_empty() {
        return Err(invalid_unit_edit_err());
    }

    accept(())
}

// Construct the error returned for invalid unit edit payloads.
fn invalid_unit_edit_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}

// Resolve a local reference id or return it as-is if it is already a real id.
fn resolve_id(
    id: String,
    local_id_map: &HashMap<String, String>,
) -> BaseRest<String> {
    //
    validate_id(&id)?;

    match local_id_map.get(&id) {
        //
        Some(resolved_id) => accept(resolved_id.clone()),

        None => accept(id),
    }
}

// Resolve a patch id, handling Clear, Assign, and Skip variants.
fn resolve_patch_id(
    value: Patch<String>,
    local_id_map: &HashMap<String, String>,
) -> BaseRest<Patch<String>> {
    match value {
        //
        Patch::Clear => accept(Patch::Clear),

        Patch::Assign(id) => {
            accept(Patch::Assign(resolve_id(id, local_id_map)?))
        }

        Patch::Skip => accept(Patch::Skip),
    }
}
