//! Presentation view for immutable chapter workflow records.

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;
use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::i18n::{trl, trl_kv};
use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::value::chapter::{Stage, StagePhase};
use crate::value::chapter_port::TranslationFormat;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordKind, ChapterWorkflowRecordPayload,
};
use crate::value::role::{RoleField, RoleMask};

/// API representation of one immutable chapter workflow record.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ChapterWorkflowRecordInfoView {
    /// Unique workflow record identifier.
    pub id: String,
    /// Chapter that owns this record.
    pub chapter_id: String,
    /// User that caused the record, absent for system work.
    pub actor_user_id: Option<String>,
    /// Stable event kind for client-side branching.
    pub kind: ChapterWorkflowRecordKind,
    /// Localized server-rendered event text without an actor name.
    pub text: String,
    /// Record creation time in Unix milliseconds.
    pub created_at: i64,
}

impl From<ChapterWorkflowRecordInfo> for ChapterWorkflowRecordInfoView {
    // Converts a read projection at the presentation and localization boundary.
    fn from(model: ChapterWorkflowRecordInfo) -> Self {
        //
        let text = render_text(&model.payload);

        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            actor_user_id: model.actor_user_id,
            kind: model.kind,
            text,
            created_at: model.created_at.to_unix_milli(),
        }
    }
}

// Builds Fluent replacement arguments from already rendered values.
fn trl_with(values: &[(&'static str, String)], key: &str) -> String {
    //
    let args = values
        .iter()
        .map(|(name, value)| {
            //
            (
                Cow::Borrowed(*name),
                FluentValue::String(Cow::Owned(value.clone())),
            )
        })
        .collect::<HashMap<_, _>>();

    trl_kv(key, &args)
}

// Renders all role bits in the deterministic production-role order.
fn role_names(roles: RoleMask) -> String {
    //
    let fields = [
        (
            RoleField::RAW_PROVIDER,
            "chapter-workflow-role-raw-provider",
        ),
        (RoleField::TRANSLATOR, "chapter-workflow-role-translator"),
        (RoleField::PROOFREADER, "chapter-workflow-role-proofreader"),
        (RoleField::TYPESETTER, "chapter-workflow-role-typesetter"),
        (RoleField::REDRAWER, "chapter-workflow-role-redrawer"),
        (RoleField::REVIEWER, "chapter-workflow-role-reviewer"),
        (RoleField::PUBLISHER, "chapter-workflow-role-publisher"),
        (RoleField::ADMIN, "chapter-workflow-role-admin"),
        (RoleField::BOT, "chapter-workflow-role-bot"),
    ];

    fields
        .iter()
        .filter(|(role, _)| roles.has_any_role(&[*role]))
        .map(|(_, key)| trl(key))
        .collect::<Vec<_>>()
        .join(", ")
}

// Looks up the localized display name of one import or export format.
fn format_name(format: TranslationFormat) -> String {
    //
    match format {
        //
        TranslationFormat::LabelPlus => {
            trl("chapter-workflow-format-label-plus")
        }

        TranslationFormat::PopRaKo => trl("chapter-workflow-format-poprako"),
    }
}

// Looks up the localized display name of one workflow stage.
fn stage_name(stage: Stage) -> String {
    //
    match stage {
        //
        Stage::RawProvide => trl("chapter-workflow-stage-raw-provide"),

        Stage::Translate => trl("chapter-workflow-stage-translate"),

        Stage::Proofread => trl("chapter-workflow-stage-proofread"),

        Stage::TypesetRedraw => trl("chapter-workflow-stage-typeset-redraw"),

        Stage::Review => trl("chapter-workflow-stage-review"),

        Stage::Publish => trl("chapter-workflow-stage-publish"),
    }
}

// Looks up the localized display name of one workflow phase.
fn phase_name(phase: StagePhase) -> String {
    //
    match phase {
        //
        StagePhase::Pending => trl("chapter-workflow-phase-pending"),

        StagePhase::Active => trl("chapter-workflow-phase-active"),

        StagePhase::Completed => trl("chapter-workflow-phase-completed"),
    }
}

// Renders a language-specific event description from language-neutral details.
fn render_text(payload: &ChapterWorkflowRecordPayload) -> String {
    //
    match payload {
        //
        ChapterWorkflowRecordPayload::ChapterCreated => {
            trl("chapter-workflow-record-created")
        }

        ChapterWorkflowRecordPayload::ChapterSubtitleUpdated {
            previous_subtitle,
            next_subtitle,
        } => trl_with(
            &[
                ("previous_subtitle", previous_subtitle.clone()),
                ("next_subtitle", next_subtitle.clone()),
            ],
            "chapter-workflow-record-subtitle-updated",
        ),

        ChapterWorkflowRecordPayload::ChapterPinned => {
            trl("chapter-workflow-record-pinned")
        }

        ChapterWorkflowRecordPayload::ChapterUnpinned => {
            trl("chapter-workflow-record-unpinned")
        }

        ChapterWorkflowRecordPayload::AssignmentCreated {
            subject_user_id: _,
            roles,
        } => trl_with(
            &[("roles", role_names(*roles))],
            "chapter-workflow-record-assignment-created",
        ),

        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
            subject_user_id: _,
            previous_roles,
            next_roles,
        } => trl_with(
            &[
                ("previous_roles", role_names(*previous_roles)),
                ("next_roles", role_names(*next_roles)),
            ],
            "chapter-workflow-record-assignment-roles-updated",
        ),

        ChapterWorkflowRecordPayload::AssignmentDeleted {
            subject_user_id: _,
            previous_roles,
        } => trl_with(
            &[("previous_roles", role_names(*previous_roles))],
            "chapter-workflow-record-assignment-deleted",
        ),

        ChapterWorkflowRecordPayload::TranslationImported {
            format,
            imported_page_count,
            imported_unit_count,
        } => trl_with(
            &[
                ("format", format_name(*format)),
                ("page_count", imported_page_count.to_string()),
                ("unit_count", imported_unit_count.to_string()),
            ],
            "chapter-workflow-record-translation-imported",
        ),

        ChapterWorkflowRecordPayload::TranslationExported { format } => {
            //
            trl_with(
                &[("format", format_name(*format))],
                "chapter-workflow-record-translation-exported",
            )
        }

        ChapterWorkflowRecordPayload::StageTransitioned {
            stage,
            previous_phase,
            next_phase,
            origin: _,
        } => trl_with(
            &[
                ("stage", stage_name(*stage)),
                ("previous_phase", phase_name(*previous_phase)),
                ("next_phase", phase_name(*next_phase)),
            ],
            "chapter-workflow-record-stage-transitioned",
        ),
    }
}
