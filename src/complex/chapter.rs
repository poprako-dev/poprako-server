//! Complex-domain opers for chapter entities — identity generation, workflow
//! stage transitions, pagination helpers, and perm gates.
//!
//! ## perm model
//!
//! Read-level access (list, get) requires the caller to be a team member of the
//! owning workset's team. Write-level access (create, update info, delete) requires
//! team admin. Workflow transitions additionally validate that the caller holds a
//! role consistent with the target stage and event.

// Workflow role validation for chapter stage transitions.
mod role;

// Domain-specific cascade helpers: delete-page cleanup and pinned chapter re-link.
/// Permission gates for chapter entity operations.
pub mod perm;

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_templates::fluent_bundle::FluentValue;

use poprako_util::i18n::{trl, trl_kv};

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::write::chapter::ChapterStageRepl;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::chapter::stage::{
    Stage, StageOper, StagePhase, try_modify_stage,
};
use crate::value::index::stored_index_to_user_index;

/// Domain opers for chapter entities: ID generation, workflow-stage
/// transition computation, and small pure helpers.
pub struct ChapterComplex;

impl ChapterComplex {
    /// Generate a unique, time-ordered chapter identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Returns the user-supplied subtitle if present and non-empty, or a
    /// generated default in the format "Ch. N" (1-based).
    pub fn subtitle_or_default(
        subtitle: Option<String>,
        index: usize,
    ) -> String {
        //
        subtitle
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_subtitle(index))
    }

    /// Compute the next [`ChapterStageRepl`] by applying a [`StageOper`]
    /// to the current [`WorkflowStage`] phase of a chapter.
    pub fn build_stage_update(
        chapter_info: &ChapterInfo,
        stage: Stage,
        oper: StageOper,
    ) -> BaseRest<ChapterStageRepl> {
        //
        let current_phase = get_phase(chapter_info, stage);

        let next_phase = try_modify_stage((stage, current_phase), oper)?;

        let chapter_stage_update = ChapterStageRepl {
            id: chapter_info.id.clone(),
            stages: chapter_info.stages.try_set_phase(stage, next_phase)?,
        };

        accept(chapter_stage_update)
    }

    /// Rejects user mutations once a chapter has been published.
    pub fn ensure_chapter_writable(chapter_info: &ChapterInfo) -> BaseRest<()> {
        //
        if chapter_info
            .stages
            .has_phase(Stage::Publish, StagePhase::Completed)
        {
            let err_message = trl("error-chapter-published-frozen");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = %chapter_info.id,
                stage = ?Stage::Publish,
                stage_phase = ?StagePhase::Completed,
                "expected error: published chapter is frozen",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(())
    }
}

// Extract the current [`StagePhase`] for a given [`Stage`] from a
// [`ChapterInfo`] record.
fn get_phase(chapter_info: &ChapterInfo, stage: Stage) -> StagePhase {
    chapter_info.stages.get_phase(stage)
}

// Generate a human-readable default subtitle for a chapter, e.g. `"Ch. 1"`.
fn default_subtitle(index: usize) -> String {
    //
    let mut args = HashMap::new();

    args.insert(
        Cow::Borrowed("number"),
        FluentValue::String(Cow::Owned(
            stored_index_to_user_index(index).to_string(),
        )),
    );

    trl_kv("chapter-default-subtitle", &args)
}
