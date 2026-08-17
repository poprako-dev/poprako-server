//! Immutable chapter workflow record kinds, payloads, and origins.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::chapter::{Stage, StagePhase};
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::RoleMask;

/// Stable kind of an immutable chapter workflow record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ChapterWorkflowRecordKind {
    /// A chapter and its initial creator assignment were created.
    ChapterCreated,

    /// A chapter subtitle changed.
    ChapterSubtitleUpdated,

    /// A chapter became the pinned chapter for its comic.
    ChapterPinned,

    /// A chapter stopped being the pinned chapter for its comic.
    ChapterUnpinned,

    /// A user received a chapter assignment.
    AssignmentCreated,

    /// An existing chapter assignment changed roles.
    AssignmentRolesUpdated,

    /// A chapter assignment was deleted.
    AssignmentDeleted,

    /// Translation content was imported into a chapter.
    TranslationImported,

    /// Translation content was successfully exported from a chapter.
    TranslationExported,

    /// A chapter workflow stage changed phase.
    StageTransitioned,
}

impl ChapterWorkflowRecordKind {
    /// Returns the stable kebab-case string persisted in the repository.
    pub fn as_str(self) -> &'static str {
        //
        match self {
            //
            Self::ChapterCreated => "chapter-created",

            Self::ChapterSubtitleUpdated => "chapter-subtitle-updated",

            Self::ChapterPinned => "chapter-pinned",

            Self::ChapterUnpinned => "chapter-unpinned",

            Self::AssignmentCreated => "assignment-created",

            Self::AssignmentRolesUpdated => "assignment-roles-updated",

            Self::AssignmentDeleted => "assignment-deleted",

            Self::TranslationImported => "translation-imported",

            Self::TranslationExported => "translation-exported",

            Self::StageTransitioned => "stage-transitioned",
        }
    }
}

/// Source operation that caused an automatic workflow-stage transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ChapterWorkflowRecordOrigin {
    /// A user explicitly advanced or reverted a stage.
    Manual,

    /// A unit edit submitted sufficient work to start a stage.
    UnitEdit,

    /// A translation import submitted sufficient work to start a stage.
    TranslationImport,

    /// A translation export started typesetting and redraw.
    TranslationExport,

    /// A delayed raw-provision upload-completeness check completed the stage.
    RawProvideCheck,
}

/// Typed, immutable details attached to a workflow record.
///
/// The enum's tagged representation is used for in-memory serde round trips.
/// Repository storage deliberately uses [`Self::to_storage_json`] so `f_kind`
/// remains separate and `f_payload` is always a JSON object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ChapterWorkflowRecordPayload {
    /// No additional details are needed for chapter creation.
    ChapterCreated,

    /// Previous and next chapter subtitles.
    ChapterSubtitleUpdated {
        /// Subtitle before the update.
        previous_subtitle: String,
        /// Subtitle after the update.
        next_subtitle: String,
    },

    /// No additional details are needed when a chapter is pinned.
    ChapterPinned,

    /// No additional details are needed when a chapter is unpinned.
    ChapterUnpinned,

    /// Assignment details for a newly assigned user.
    AssignmentCreated {
        /// User receiving the assignment.
        subject_user_id: String,
        /// Initial assignment roles.
        roles: RoleMask,
    },

    /// Assignment role-mask change details.
    AssignmentRolesUpdated {
        /// User whose roles changed.
        subject_user_id: String,
        /// Roles before the update.
        previous_roles: RoleMask,
        /// Roles after the update.
        next_roles: RoleMask,
    },

    /// Assignment details retained before deletion.
    AssignmentDeleted {
        /// User whose assignment was removed.
        subject_user_id: String,
        /// Roles the assignment had before deletion.
        previous_roles: RoleMask,
    },

    /// Summary of a successful translation import.
    TranslationImported {
        /// Imported content format.
        format: TranslationFormat,
        /// Number of source pages imported.
        imported_page_count: i32,
        /// Number of source units imported.
        imported_unit_count: i32,
    },

    /// Format of successfully generated translation export content.
    TranslationExported {
        /// Generated content format.
        format: TranslationFormat,
    },

    /// One real workflow-stage phase transition.
    StageTransitioned {
        /// Changed stage.
        stage: Stage,
        /// Phase before the transition.
        previous_phase: StagePhase,
        /// Phase after the transition.
        next_phase: StagePhase,
        /// Operation that originated this transition.
        origin: ChapterWorkflowRecordOrigin,
    },
}

impl ChapterWorkflowRecordPayload {
    /// Returns the stable record kind associated with this payload.
    pub fn kind(&self) -> ChapterWorkflowRecordKind {
        //
        match self {
            //
            Self::ChapterCreated => ChapterWorkflowRecordKind::ChapterCreated,

            Self::ChapterSubtitleUpdated { .. } => {
                ChapterWorkflowRecordKind::ChapterSubtitleUpdated
            }

            Self::ChapterPinned => ChapterWorkflowRecordKind::ChapterPinned,

            Self::ChapterUnpinned => ChapterWorkflowRecordKind::ChapterUnpinned,

            Self::AssignmentCreated { .. } => {
                ChapterWorkflowRecordKind::AssignmentCreated
            }

            Self::AssignmentRolesUpdated { .. } => {
                ChapterWorkflowRecordKind::AssignmentRolesUpdated
            }

            Self::AssignmentDeleted { .. } => {
                ChapterWorkflowRecordKind::AssignmentDeleted
            }

            Self::TranslationImported { .. } => {
                ChapterWorkflowRecordKind::TranslationImported
            }

            Self::TranslationExported { .. } => {
                ChapterWorkflowRecordKind::TranslationExported
            }

            Self::StageTransitioned { .. } => {
                ChapterWorkflowRecordKind::StageTransitioned
            }
        }
    }

    /// Serializes this payload as the object stored in the `f_payload` column.
    pub fn to_storage_json(&self) -> Value {
        //
        match self {
            //
            Self::ChapterCreated
            | Self::ChapterPinned
            | Self::ChapterUnpinned => {
                json!({})
            }

            Self::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            } => {
                //
                json!({
                    "previous_subtitle": previous_subtitle,
                    "next_subtitle": next_subtitle,
                })
            }

            Self::AssignmentCreated {
                subject_user_id,
                roles,
            } => {
                json!({ "subject_user_id": subject_user_id, "roles": roles })
            }

            Self::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } => {
                //
                json!({
                    "subject_user_id": subject_user_id,
                    "previous_roles": previous_roles,
                    "next_roles": next_roles,
                })
            }

            Self::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            } => {
                //
                json!({
                    "subject_user_id": subject_user_id,
                    "previous_roles": previous_roles,
                })
            }

            Self::TranslationImported {
                format,
                imported_page_count,
                imported_unit_count,
            } => {
                //
                json!({
                    "format": format,
                    "imported_page_count": imported_page_count,
                    "imported_unit_count": imported_unit_count,
                })
            }

            Self::TranslationExported { format } => {
                json!({ "format": format })
            }

            Self::StageTransitioned {
                stage,
                previous_phase,
                next_phase,
                origin,
            } => {
                //
                json!({
                    "stage": stage,
                    "previous_phase": previous_phase,
                    "next_phase": next_phase,
                    "origin": origin,
                })
            }
        }
    }

    /// Decodes one persisted JSON object using its separate record kind.
    pub fn from_storage_json(
        kind: ChapterWorkflowRecordKind,
        payload: Value,
    ) -> Result<Self, serde_json::Error> {
        // Reconstitute the in-memory tag from the separately persisted kind.
        let mut payload_fields = match payload {
            //
            Value::Object(payload_fields) => payload_fields,

            payload => return serde_json::from_value(payload),
        };

        let expected_fields = match kind {
            //
            ChapterWorkflowRecordKind::ChapterCreated
            | ChapterWorkflowRecordKind::ChapterPinned
            | ChapterWorkflowRecordKind::ChapterUnpinned => &[][..],

            ChapterWorkflowRecordKind::ChapterSubtitleUpdated => {
                &["previous_subtitle", "next_subtitle"][..]
            }

            ChapterWorkflowRecordKind::AssignmentCreated => {
                &["subject_user_id", "roles"][..]
            }

            ChapterWorkflowRecordKind::AssignmentRolesUpdated => {
                &["subject_user_id", "previous_roles", "next_roles"][..]
            }

            ChapterWorkflowRecordKind::AssignmentDeleted => {
                &["subject_user_id", "previous_roles"][..]
            }

            ChapterWorkflowRecordKind::TranslationImported => {
                &["format", "imported_page_count", "imported_unit_count"][..]
            }

            ChapterWorkflowRecordKind::TranslationExported => &["format"][..],

            ChapterWorkflowRecordKind::StageTransitioned => {
                &["stage", "previous_phase", "next_phase", "origin"][..]
            }
        };

        let has_expected_fields = payload_fields.len() == expected_fields.len()
            && expected_fields
                .iter()
                .all(|field_name| payload_fields.contains_key(*field_name));

        if !has_expected_fields {
            //
            return Err(serde_json::Error::io(std::io::Error::other(
                "persisted workflow record payload has invalid fields",
            )));
        }

        payload_fields
            .insert("type".into(), Value::String(kind.as_str().into()));

        serde_json::from_value(Value::Object(payload_fields))
    }
}
