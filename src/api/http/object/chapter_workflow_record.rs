//! HTTP wire objects for the migrated data DTOs.
use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::result::Wire;
use crate::data::view::chapter_workflow_record::{
    ChapterWorkflowRecordEventView, ChapterWorkflowRecordInfoView,
    ChapterWorkflowRecordOriginView, ChapterWorkflowRecordStagePhaseView,
    ChapterWorkflowRecordStageView, ChapterWorkflowRecordTranslationFormatView,
};
use crate::value::chapter_port::ExportFormatSpec;
use crate::value::role::RoleMask;

/// API representation of one immutable chapter workflow record.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterWorkflowRecordInfoObj {
    /// Unique workflow record identifier.
    pub id: String,

    /// Chapter that owns this record.
    pub chapter_id: String,
    /// User that caused the record, absent for system work.
    pub actor_user_id: Option<String>,

    /// Strongly typed, language-neutral event data for client-side rendering.
    pub event: ChapterWorkflowRecordEventObj,

    /// Record creation time in Unix milliseconds.
    pub created_at: i64,
}

impl From<ChapterWorkflowRecordInfoView> for ChapterWorkflowRecordInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordInfoView) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            actor_user_id: value.actor_user_id,
            event: value.event.into(),
            created_at: value.created_at,
        }
    }
}

impl Wire for ChapterWorkflowRecordInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Strongly typed workflow event exposed to clients as a discriminated union.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ChapterWorkflowRecordEventObj {
    /// A chapter and its initial creator assignment were created.
    ChapterCreated,

    /// A chapter subtitle changed.
    ChapterSubtitleUpdated {
        /// Subtitle before the update.
        previous_subtitle: String,
        /// Subtitle after the update.
        next_subtitle: String,
    },

    /// A chapter became the pinned chapter for its comic.
    ChapterPinned,

    /// A chapter stopped being the pinned chapter for its comic.
    ChapterUnpinned,

    /// A user received a chapter assignment.
    AssignmentCreated {
        /// User receiving the assignment.
        subject_user_id: String,
        /// Initial assignment roles.
        roles: RoleMask,
    },

    /// An existing chapter assignment changed roles.
    AssignmentRolesUpdated {
        /// User whose roles changed.
        subject_user_id: String,
        /// Roles before the update.
        previous_roles: RoleMask,
        /// Roles after the update.
        next_roles: RoleMask,
    },

    /// A chapter assignment was deleted.
    AssignmentDeleted {
        /// User whose assignment was removed.
        subject_user_id: String,
        /// Roles before deletion.
        previous_roles: RoleMask,
    },

    /// Translation content was imported into a chapter.
    TranslationImported {
        /// Imported content format.
        format: ChapterWorkflowRecordTranslationFormatObj,
        /// Number of pages whose visible Unit content changed.
        imported_page_count: usize,
        /// Number of Units created from the imported source.
        imported_unit_count: usize,
    },

    /// Translation content was successfully exported from a chapter.
    TranslationExported {
        /// Generated content formats.
        formats: ExportFormatSpec,
    },

    /// Chapter artwork was successfully exported.
    ArtworkExported {
        /// Exported artwork generation.
        #[serde(rename = "artwork_version")]
        artwork_ver: u32,
    },

    /// A chapter workflow stage changed phase.
    StageTransitioned {
        /// Changed workflow stage.
        stage: ChapterWorkflowRecordStageObj,
        /// Phase before the transition.
        previous_phase: ChapterWorkflowRecordStagePhaseObj,
        /// Phase after the transition.
        next_phase: ChapterWorkflowRecordStagePhaseObj,
        /// Action that originated the transition.
        origin: ChapterWorkflowRecordOriginObj,
    },
}

impl From<ChapterWorkflowRecordEventView> for ChapterWorkflowRecordEventObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordEventView) -> Self {
        //
        match value {
            //
            ChapterWorkflowRecordEventView::ChapterCreated => {
                Self::ChapterCreated
            }

            ChapterWorkflowRecordEventView::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            } => Self::ChapterSubtitleUpdated {
                previous_subtitle,
                next_subtitle,
            },

            ChapterWorkflowRecordEventView::ChapterPinned => {
                Self::ChapterPinned
            }

            ChapterWorkflowRecordEventView::ChapterUnpinned => {
                Self::ChapterUnpinned
            }

            ChapterWorkflowRecordEventView::AssignmentCreated {
                subject_user_id,
                roles,
            } => Self::AssignmentCreated {
                subject_user_id,
                roles,
            },

            ChapterWorkflowRecordEventView::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            } => Self::AssignmentRolesUpdated {
                subject_user_id,
                previous_roles,
                next_roles,
            },

            ChapterWorkflowRecordEventView::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            } => Self::AssignmentDeleted {
                subject_user_id,
                previous_roles,
            },

            ChapterWorkflowRecordEventView::TranslationImported {
                format,
                imported_page_count,
                imported_unit_count,
            } => Self::TranslationImported {
                format: format.into(),
                imported_page_count,
                imported_unit_count,
            },

            ChapterWorkflowRecordEventView::TranslationExported { formats } => {
                Self::TranslationExported { formats }
            }

            ChapterWorkflowRecordEventView::ArtworkExported { artwork_ver } => {
                Self::ArtworkExported { artwork_ver }
            }

            ChapterWorkflowRecordEventView::StageTransitioned {
                stage,
                previous_phase,
                next_phase,
                origin,
            } => Self::StageTransitioned {
                stage: stage.into(),
                previous_phase: previous_phase.into(),
                next_phase: next_phase.into(),
                origin: origin.into(),
            },
        }
    }
}

impl Wire for ChapterWorkflowRecordEventView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordEventObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Translation format used by a workflow record event.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordTranslationFormatObj {
    /// `LabelPlus` translation format.
    LabelPlus,

    /// `Poprako` native translation format.
    Poprako,
}

impl From<ChapterWorkflowRecordTranslationFormatView>
    for ChapterWorkflowRecordTranslationFormatObj
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordTranslationFormatView) -> Self {
        //
        match value {
            //
            ChapterWorkflowRecordTranslationFormatView::LabelPlus => {
                Self::LabelPlus
            }

            ChapterWorkflowRecordTranslationFormatView::Poprako => {
                Self::Poprako
            }
        }
    }
}

impl Wire for ChapterWorkflowRecordTranslationFormatView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordTranslationFormatObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Workflow stage used by a workflow record event.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordStageObj {
    /// Raw-provision stage.
    RawProvide,

    /// Translation stage.
    Translate,

    /// Proofreading stage.
    Proofread,

    /// Typesetting and redraw stage.
    TypesetRedraw,

    /// Review stage.
    Review,

    /// Publishing stage.
    Publish,
}

impl From<ChapterWorkflowRecordStageView> for ChapterWorkflowRecordStageObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordStageView) -> Self {
        //
        match value {
            //
            ChapterWorkflowRecordStageView::RawProvide => Self::RawProvide,

            ChapterWorkflowRecordStageView::Translate => Self::Translate,

            ChapterWorkflowRecordStageView::Proofread => Self::Proofread,

            ChapterWorkflowRecordStageView::TypesetRedraw => {
                Self::TypesetRedraw
            }

            ChapterWorkflowRecordStageView::Review => Self::Review,

            ChapterWorkflowRecordStageView::Publish => Self::Publish,
        }
    }
}

impl Wire for ChapterWorkflowRecordStageView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordStageObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Workflow-stage phase used by a workflow record event.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordStagePhaseObj {
    /// The stage has not started.
    Pending,

    /// The stage is in progress.
    Active,

    /// The stage has finished.
    Completed,
}

impl From<ChapterWorkflowRecordStagePhaseView>
    for ChapterWorkflowRecordStagePhaseObj
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordStagePhaseView) -> Self {
        //
        match value {
            //
            ChapterWorkflowRecordStagePhaseView::Pending => Self::Pending,

            ChapterWorkflowRecordStagePhaseView::Active => Self::Active,

            ChapterWorkflowRecordStagePhaseView::Completed => Self::Completed,
        }
    }
}

impl Wire for ChapterWorkflowRecordStagePhaseView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordStagePhaseObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Operation source used by a workflow record event.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum ChapterWorkflowRecordOriginObj {
    /// Explicit stage operation.
    Manual,

    /// Unit mutation.
    UnitEdit,

    /// Translation import.
    TranslationImport,

    /// Translation export.
    TranslationExport,

    /// Raw-provision completeness check.
    RawProvideCheck,

    /// Client confirmation of the current artwork upload.
    ArtworkUpload,
}

impl From<ChapterWorkflowRecordOriginView> for ChapterWorkflowRecordOriginObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ChapterWorkflowRecordOriginView) -> Self {
        //
        match value {
            //
            ChapterWorkflowRecordOriginView::Manual => Self::Manual,

            ChapterWorkflowRecordOriginView::UnitEdit => Self::UnitEdit,

            ChapterWorkflowRecordOriginView::TranslationImport => {
                Self::TranslationImport
            }

            ChapterWorkflowRecordOriginView::TranslationExport => {
                Self::TranslationExport
            }

            ChapterWorkflowRecordOriginView::RawProvideCheck => {
                Self::RawProvideCheck
            }

            ChapterWorkflowRecordOriginView::ArtworkUpload => {
                Self::ArtworkUpload
            }
        }
    }
}

impl Wire for ChapterWorkflowRecordOriginView {
    // Serializable wire payload produced by this conversion.
    type Target = ChapterWorkflowRecordOriginObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
