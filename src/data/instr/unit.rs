//! Instr DTOs for the unit domain.

//! Data transfer objects for page Unit use cases.
//!
//! Types in this module describe how client-supplied edit payloads are
//! represented and how persisted Unit rows are projected back into API-facing
//! response types.

use std::collections::HashMap;

use serde::Deserialize;

use poprako_util::i18n::trl;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::Patch;

#[cfg(test)]
mod tests;

/// Input parameters for listing visible Units under one Page.
#[derive(Debug, Deserialize)]
pub struct ListPageUnitInfosInstr {
    /// Page whose visible Units are listed.
    pub page_id: String,
}

/// Input parameters for saving a batch of Unit edits.
#[derive(Debug, Deserialize)]
pub struct SavePageUnitEditsInstr {
    //
    /// Page whose Units are being edited.
    pub page_id: String,

    /// Ordered batch of Unit edits.
    pub edits: Vec<UnitEditInstr>,
}

/// One transport-facing Unit edit.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitEditInstr {
    //
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
        coord: UnitCoordInstr,

        /// Optional initial translation assignment.
        #[serde(default)]
        translation: Option<UnitTranslationInstr>,
        /// Optional initial revision assignment.
        #[serde(default)]
        revision: Option<UnitRevisionInstr>,
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
        coord: Option<UnitCoordInstr>,

        /// Three-state translation patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<UnitTranslationInstr>)
        )]
        translation: Patch<UnitTranslationInstr>,
        /// Three-state revision patch.
        #[serde(default)]
        #[cfg_attr(
            feature = "swagger",
            schema(value_type = Option<UnitRevisionInstr>)
        )]
        revision: Patch<UnitRevisionInstr>,
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
pub struct UnitCoordInstr {
    //
    /// Horizontal page-relative coordinate.
    pub x_coord: f64,
    /// Vertical page-relative coordinate.
    pub y_coord: f64,
}

impl From<UnitCoordInstr> for UnitCoord {
    // Convert API coordinate value into domain coordinate.
    fn from(value: UnitCoordInstr) -> Self {
        //
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
pub struct UnitTranslationInstr {
    /// Replacement translated text.
    pub translated_text: String,
}

/// Revision assignment accepted from the client.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(deny_unknown_fields)]
pub struct UnitRevisionInstr {
    //
    /// Replacement approval state.
    pub is_proofread: bool,

    /// Replacement proofread text.
    #[serde(default)]
    pub proofread_text: Option<String>,
}

impl UnitEditInstr {
    // Convert one transport edit operation into a repository edit command.
    fn into_model(
        self,
        user_id: &str,
        local_id_map: &HashMap<String, String>,
    ) -> BaseRest<UnitEdit> {
        //
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

/// Converts transport edits into domain Saves and Deletes.
///
/// The returned list preserves input order and maps request-local references.
pub fn into_unit_edits<F>(
    edits: Vec<UnitEditInstr>,
    user_id: &str,
    mut gen_id: F,
) -> BaseRest<Vec<UnitEdit>>
where
    F: FnMut() -> String,
{
    let mut local_id_map = HashMap::new();

    for edit in &edits {
        //
        let UnitEditInstr::Create { local_id, .. } = edit else {
            continue;
        };

        validate_id(local_id)?;

        let unit_id = gen_id();

        if local_id_map.insert(local_id.clone(), unit_id).is_some() {
            //
            let err_message = trl("error-invalid-unit-oper");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                local_id = %local_id,
                "expected error: duplicate unit edit local id",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }
    }

    let edits = edits
        .into_iter()
        .map(|edit| edit.into_model(user_id, &local_id_map))
        .collect::<BaseRest<Vec<_>>>()?;

    accept(edits)
}

// Validate a unit edit local reference id is non-empty.
fn validate_id(id: &str) -> BaseRest<()> {
    //
    if id.is_empty() {
        //
        let err_message = trl("error-invalid-unit-oper");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            local_or_unit_id = %id,
            "expected error: empty unit edit id",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
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
    //
    match value {
        //
        Patch::Clear => accept(Patch::Clear),

        Patch::Assign(id) => {
            accept(Patch::Assign(resolve_id(id, local_id_map)?))
        }

        Patch::Skip => accept(Patch::Skip),
    }
}
