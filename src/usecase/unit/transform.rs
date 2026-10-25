//! Chapter-scoped literal Unit text transformation.

use std::collections::{HashMap, HashSet};

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::unit::perm as unit_perm_complex;
use crate::complex::{chapter as chapter_complex, unit as unit_complex};
use crate::data::instr::unit::{
    TransformChapterUnitsInstr, into_unit_transforms,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::read::proj::unit::{UnitCountDelta, UnitInfo};
use crate::model::shared::user::UserToken;
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::part::nucl::Serial;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCountDelta, GetChapterInfoExcluded,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    GetPageSourceUnitScopeExcluded, ListPageSourceInfos,
};
use crate::part::repo::oper::unit::ListUnitInfosByIds;
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::usecase::stage as stage_usecase;
use crate::usecase::unit::page_source_edit as unit_page_source_edit_usecase;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::unit::UnitTextPart;

/// Builds the client-visible error for an invalid Chapter Unit transform.
pub fn invalid_unit_transform(
    chapter_id: &str,
    unit_id: &str,
    reason: &'static str,
) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-invalid-unit-transform");
        chapter_id,
        unit_id,
        reason,
        "invalid chapter unit transform",
    }
}

/// Adds one page counter delta to a Chapter-level aggregate.
pub const fn add_count_delta(
    total: &mut UnitCountDelta,
    delta: UnitCountDelta,
) {
    //
    total.total += delta.total;

    total.translated += delta.translated;

    total.proofread += delta.proofread;
}

/// Builds the current content edits selected for one source page.
pub fn build_page_source_edits(
    page_source_id: &str,
    part: UnitTextPart,
    unit_transforms: &[UnitTransform],
    unit_infos: &HashMap<&str, &UnitInfo>,
    user_id: &str,
) -> BaseRest<Vec<UnitEdit>> {
    //
    let mut edits = Vec::new();

    for unit_transform in unit_transforms {
        //
        let Some(unit_info) = unit_infos.get(unit_transform.unit_id.as_str())
        else {
            continue;
        };

        if unit_info.page_source_id != page_source_id
            || unit_info.hidden_at.is_some()
        {
            continue;
        }

        let edit = unit_complex::build_transform_edit(
            unit_info,
            part,
            unit_transform,
            user_id,
        )?;

        let Some(edit) = edit else {
            continue;
        };

        edits.push(edit);
    }

    accept!(edits)
}

// Groups the Chapter-scoped models used by a Unit transformation.
struct TransformScope {
    // Chapter being transformed.
    chapter_info: ChapterInfo,
    // PageSources belonging to the Chapter.
    page_source_infos: Vec<PageSourceInfo>,
    // Selected Units indexed by their persisted IDs.
    unit_infos: Vec<UnitInfo>,
}

#[instrument(
    level = "info",
    skip(nucl, repo, token, instr),
    fields(
        actor_user_id = %token.user_id,
        part = ?instr.part,
        unit_count = instr.units.len(),
    ),
)]
/// Transforms one Unit text field across selected Units in a Chapter.
pub async fn transform<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
    instr: TransformChapterUnitsInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + PageSourceRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let TransformChapterUnitsInstr { part, units } = instr;

    let unit_transforms = into_unit_transforms(units)?;

    let () = nucl
        .coord(async move |context| {
            //
            let unit_ids = unit_transforms
                .iter()
                .map(|unit_transform| unit_transform.unit_id.as_str())
                .collect::<Vec<_>>();

            let transform_scope = load_transform_scope(
                repo,
                context,
                &chapter_id,
                &token.user_id,
                TransformTargets {
                    part,
                    unit_ids: &unit_ids,
                },
            )
            .await?;

            let unit_infos = transform_scope
                .unit_infos
                .iter()
                .map(|unit_info| (unit_info.id.as_str(), unit_info))
                .collect::<HashMap<_, _>>();

            let mut total_delta = UnitCountDelta::default();

            let mut applied_edits = Vec::new();

            for page_source_info in &transform_scope.page_source_infos {
                //
                let page_source_transform = apply_page_source_transforms(
                    repo,
                    context,
                    &page_source_info.id,
                    PageSourceTransformData {
                        part,
                        unit_transforms: &unit_transforms,
                        unit_infos: &unit_infos,
                    },
                    &token.user_id,
                )
                .await?;

                let Some((count_delta, edits)) = page_source_transform else {
                    continue;
                };

                add_count_delta(&mut total_delta, count_delta);

                applied_edits.extend(edits);
            }

            finish_transform(
                repo,
                context,
                &transform_scope.chapter_info,
                &token,
                TransformChanges {
                    total_delta,
                    applied_edits: &applied_edits,
                },
            )
            .await
        })
        .await?;

    accept!(())
}

// Selects the text part and Units to transform.
struct TransformTargets<'a> {
    // Text field selected for transformation.
    part: UnitTextPart,
    // Unit IDs submitted by the caller.
    unit_ids: &'a [&'a str],
}

// Locks and validates the Chapter-scoped models selected for transformation.
async fn load_transform_scope<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
    targets: TransformTargets<'_>,
) -> BaseRest<TransformScope>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + PageSourceRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let TransformTargets { part, unit_ids } = targets;

    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    chapter_complex::ensure_chapter_writable(&chapter_info)?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &chapter_info.id,
        user_id,
    }
    .step_on(repo, context)
    .await?;

    let edit_perm = unit_perm_complex::edit_perm(assignment_info.as_ref());

    unit_perm_complex::ensure_user_can_transform(edit_perm, part)?;

    let page_source_infos = ListPageSourceInfos {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let unit_infos = ListUnitInfosByIds { ids: unit_ids }
        .step_on(repo, context)
        .await?;

    let chapter_page_source_ids = page_source_infos
        .iter()
        .map(|page_source_info| page_source_info.id.as_str())
        .collect::<HashSet<_>>();

    for unit_info in &unit_infos {
        //
        if !chapter_page_source_ids.contains(unit_info.page_source_id.as_str())
        {
            //
            return Err(invalid_unit_transform(
                &chapter_info.id,
                &unit_info.id,
                "cross_chapter_unit",
            ));
        }
    }

    accept!(TransformScope {
        chapter_info,
        page_source_infos,
        unit_infos,
    })
}

// Prepared Unit transforms and their current models for one PageSource.
struct PageSourceTransformData<'a> {
    // Text field selected for transformation.
    part: UnitTextPart,
    // Requested edits for the selected Units.
    unit_transforms: &'a [UnitTransform],
    // Persisted Unit projections indexed by ID.
    unit_infos: &'a HashMap<&'a str, &'a UnitInfo>,
}

// Applies the selected text transforms and counter update to one PageSource.
async fn apply_page_source_transforms<C, R>(
    repo: &R,
    context: &mut C,
    page_source_id: &str,
    data: PageSourceTransformData<'_>,
    user_id: &str,
) -> BaseRest<Option<(UnitCountDelta, Vec<UnitEdit>)>>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: PageSourceRepo<C> + UnitRepo<C> + Send + Sync,
{
    let PageSourceTransformData {
        part,
        unit_transforms,
        unit_infos,
    } = data;

    let edits = build_page_source_edits(
        page_source_id,
        part,
        unit_transforms,
        unit_infos,
        user_id,
    )?;

    if edits.is_empty() {
        return accept!(None);
    }

    let page_source_scope =
        GetPageSourceUnitScopeExcluded { id: page_source_id }
            .step_on(repo, context)
            .await?;

    let page_source_edit = unit_page_source_edit_usecase::apply_edits(
        repo,
        context,
        &page_source_scope,
        edits,
    )
    .await?;

    accept!(Some(page_source_edit.into_changes()))
}

// Aggregates persisted Unit changes across the transformed PageSources.
struct TransformChanges<'a> {
    // Combined Chapter count change across PageSources.
    total_delta: UnitCountDelta,
    // Unit edits that were actually applied.
    applied_edits: &'a [UnitEdit],
}

// Applies Chapter aggregates and workflow effects after Unit edits.
async fn finish_transform<C, R>(
    repo: &R,
    context: &mut C,
    chapter_info: &ChapterInfo,
    token: &UserToken,
    changes: TransformChanges<'_>,
) -> BaseRest<()>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + Send
        + Sync,
{
    let TransformChanges {
        total_delta,
        applied_edits,
    } = changes;

    if applied_edits.is_empty() {
        return accept!(());
    }

    AdjustChapterUnitCountDelta {
        id: &chapter_info.id,
        delta: total_delta,
    }
    .step_on(repo, context)
    .await?;

    TouchComicLastActive {
        id: &chapter_info.comic_id,
    }
    .step_on(repo, context)
    .await?;

    let stages = unit_complex::submitted_stage_advances(applied_edits);

    stage_usecase::start_pending_stages(
        repo,
        context,
        stage_usecase::PendingStageStart::new(
            &chapter_info.id,
            Some(token.user_id.as_str()),
            ChapterWorkflowRecordOrigin::UnitEdit,
        ),
        &stages,
    )
    .await?;

    accept!(())
}
