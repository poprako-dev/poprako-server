//! Strongly typed chapter workflow details retained in comic archives.

use serde::Serialize;

use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_port::{ExportFormatSpec, TranslationFormat};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleMask;

/// Strongly typed workflow details retained inside an archived chapter.
#[derive(Serialize)]
#[serde(untagged)]
pub enum ArchivedChapterWorkflowRecordDetail {
    //
    /// No details for chapter creation, pinning, or unpinning.
    Empty {},

    /// Previous and next chapter subtitles.
    SubtitleUpdated {
        /// Subtitle before the update.
        previous_subtitle: String,
        /// Subtitle after the update.
        next_subtitle: String,
    },

    /// Initial assignment details.
    AssignmentCreated {
        /// User receiving the assignment.
        subject_user_id: String,
        /// Initial assignment roles.
        roles: RoleMask,
    },

    /// Assignment role-mask transition.
    AssignmentRolesUpdated {
        /// User whose assignment changed.
        subject_user_id: String,
        /// Roles before the update.
        previous_roles: RoleMask,
        /// Roles after the update.
        next_roles: RoleMask,
    },

    /// Assignment details retained before deletion.
    AssignmentDeleted {
        /// User whose assignment was deleted.
        subject_user_id: String,
        /// Roles before deletion.
        previous_roles: RoleMask,
    },

    /// Imported translation summary.
    TranslationImported {
        /// Imported content format.
        format: TranslationFormat,
        /// Number of imported pages.
        imported_page_count: usize,
        /// Number of imported units.
        imported_unit_count: usize,
    },

    /// Exported translation formats.
    TranslationExported {
        /// Generated content formats.
        formats: ExportFormatSpec,
    },

    /// Workflow-stage phase transition.
    StageTransitioned {
        /// Changed workflow stage.
        stage: Stage,
        /// Phase before the transition.
        previous_phase: StagePhase,
        /// Phase after the transition.
        next_phase: StagePhase,
        /// Operation that caused the transition.
        origin: ChapterWorkflowRecordOrigin,
    },
}

impl From<&ChapterWorkflowRecordPayload>
    for ArchivedChapterWorkflowRecordDetail
{
    // Converts domain workflow details into the stable archive shape.
    fn from(payload: &ChapterWorkflowRecordPayload) -> Self {
        //
        match payload {
            //
            ChapterWorkflowRecordPayload::ChapterCreated
            | ChapterWorkflowRecordPayload::ChapterPinned
            | ChapterWorkflowRecordPayload::ChapterUnpinned => Self::Empty {},

            ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            } => Self::SubtitleUpdated {
                previous_subtitle: previous_subtitle.clone(),
                next_subtitle: next_subtitle.clone(),
            },

            ChapterWorkflowRecordPayload::AssignmentCreated {
                subject_user_id,
                roles,
            } => Self::AssignmentCreated {
                subject_user_id: subject_user_id.clone(),
                roles: *roles,
            },

            ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } => Self::AssignmentRolesUpdated {
                subject_user_id: subject_user_id.clone(),
                previous_roles: *previous_roles,
                next_roles: *next_roles,
            },

            ChapterWorkflowRecordPayload::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            } => Self::AssignmentDeleted {
                subject_user_id: subject_user_id.clone(),
                previous_roles: *previous_roles,
            },

            ChapterWorkflowRecordPayload::TranslationImported {
                format,
                imported_page_count,
                imported_unit_count,
            } => Self::TranslationImported {
                format: *format,
                imported_page_count: *imported_page_count,
                imported_unit_count: *imported_unit_count,
            },

            ChapterWorkflowRecordPayload::TranslationExported { formats } => {
                Self::TranslationExported { formats: *formats }
            }

            ChapterWorkflowRecordPayload::StageTransitioned {
                stage,
                previous_phase,
                next_phase,
                origin,
            } => Self::StageTransitioned {
                stage: *stage,
                previous_phase: *previous_phase,
                next_phase: *next_phase,
                origin: *origin,
            },
        }
    }
}
