//! Chapter-scoped literal Unit text transformation.

use std::collections::{HashMap, HashSet};

use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::UnitComplex;
use crate::complex::unit::perm::UnitPermComplex;
use crate::data::instr::unit::{
    TransformChapterUnitsInstr, into_unit_transforms,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::{
    UnitCountDelta, UnitCountMetrics, UnitInfo,
};
use crate::model::shared::user::UserToken;
use crate::model::write::unit::{UnitEdit, UnitTransform};
use crate::part::nucl::Serial;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, GetChapterInfoExcluded,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, ListPageInfos, SetPageUnitCounters,
};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitOrders,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::role::RoleField;
use crate::value::unit::{UnitEditPerm, UnitTextPart};

/// Builds the client-visible error for an invalid Chapter Unit transform.
pub fn invalid_unit_transform(
    chapter_id: &str,
    unit_id: &str,
    reason: &'static str,
) -> BaseError {
    //
    let err_message = trl("error-invalid-unit-transform");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        chapter_id,
        unit_id,
        reason,
        "expected error: invalid chapter unit transform",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

/// Adds one page counter delta to a Chapter-level aggregate.
pub const fn add_counter_delta(
    total: &mut UnitCountDelta,
    delta: UnitCountDelta,
) {
    //
    total.total += delta.total;

    total.translated += delta.translated;

    total.proofread += delta.proofread;
}

/// Builds the current content edits selected for one Page.
pub fn build_page_edits(
    page_id: &str,
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

        if unit_info.page_id != page_id || unit_info.hidden_at.is_some() {
            continue;
        }

        let edit = UnitComplex::build_transform_edit(
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

    accept(edits)
}

// Groups the Chapter-scoped models used by a Unit transformation.
struct TransformScope {
    //
    // Chapter being transformed.
    chapter_info: ChapterInfo,
    // Pages belonging to the Chapter.
    page_infos: Vec<PageInfo>,
    // Selected Units indexed by their persisted IDs.
    unit_infos: Vec<UnitInfo>,
}

#[instrument(
    level = "info",
    skip(nucl, repo, token),
    fields(
        actor_user_id = %token.user_id,
        chapter_id = %chapter_id,
        part = ?instr.part,
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
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let TransformChapterUnitsInstr { part, units } = instr;

    let unit_transforms = into_unit_transforms(units)?;

    let unit_ids = unit_transforms
        .iter()
        .map(|unit_transform| unit_transform.unit_id.clone())
        .collect::<Vec<_>>();

    let () = nucl
        .coord(async move |context| {
            //
            let transform_scope = load_transform_scope(
                repo,
                context,
                &chapter_id,
                &token.user_id,
                part,
                &unit_ids,
            )
            .await?;

            let unit_infos = transform_scope
                .unit_infos
                .iter()
                .map(|unit_info| (unit_info.id.as_str(), unit_info))
                .collect::<HashMap<_, _>>();

            let mut total_delta = UnitCountDelta::default();

            let mut applied_edits = Vec::new();

            for page_info in &transform_scope.page_infos {
                //
                let page_transform = apply_page_transforms(
                    repo,
                    context,
                    &page_info.id,
                    part,
                    &unit_transforms,
                    &unit_infos,
                    &token.user_id,
                )
                .await?;

                let Some((counter_delta, edits)) = page_transform else {
                    continue;
                };

                add_counter_delta(&mut total_delta, counter_delta);

                applied_edits.extend(edits);
            }

            finish_transform(
                repo,
                context,
                &transform_scope.chapter_info,
                &token,
                total_delta,
                &applied_edits,
            )
            .await
        })
        .await?;

    accept(())
}

// Locks and validates the Chapter-scoped models selected for transformation.
async fn load_transform_scope<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
    part: UnitTextPart,
    unit_ids: &[String],
) -> BaseRest<TransformScope>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    ChapterComplex::ensure_chapter_writable(&chapter_info)?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &chapter_info.id,
        user_id,
    }
    .step_on(repo, context)
    .await?;

    let edit_perm = UnitEditPerm {
        can_translate: assignment_info.as_ref().is_some_and(
            |assignment_info| {
                assignment_info.roles.has_any_role(&[RoleField::TRANSLATOR])
            },
        ),
        can_proofread: assignment_info.as_ref().is_some_and(
            |assignment_info| {
                //
                assignment_info
                    .roles
                    .has_any_role(&[RoleField::PROOFREADER])
            },
        ),
    };

    UnitPermComplex::ensure_user_can_transform(edit_perm, part)?;

    let page_infos = ListPageInfos {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let unit_infos = ListUnitInfosByIds { ids: unit_ids }
        .step_on(repo, context)
        .await?;

    let chapter_page_ids = page_infos
        .iter()
        .map(|page_info| page_info.id.as_str())
        .collect::<HashSet<_>>();

    for unit_info in &unit_infos {
        //
        if !chapter_page_ids.contains(unit_info.page_id.as_str()) {
            //
            return Err(invalid_unit_transform(
                &chapter_info.id,
                &unit_info.id,
                "cross_chapter_unit",
            ));
        }
    }

    accept(TransformScope {
        chapter_info,
        page_infos,
        unit_infos,
    })
}

// Applies the selected text transforms and counter update to one Page.
async fn apply_page_transforms<C, R>(
    repo: &R,
    context: &mut C,
    page_id: &str,
    part: UnitTextPart,
    unit_transforms: &[UnitTransform],
    unit_infos: &HashMap<&str, &UnitInfo>,
    user_id: &str,
) -> BaseRest<Option<(UnitCountDelta, Vec<UnitEdit>)>>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: PageRepo<C> + UnitRepo<C> + Send + Sync,
{
    let edits =
        build_page_edits(page_id, part, unit_transforms, unit_infos, user_id)?;

    if edits.is_empty() {
        return accept(None);
    }

    let page_info = GetPageInfoExcluded { id: page_id }
        .step_on(repo, context)
        .await?;

    let orders = ListUnitOrders {
        page_id: &page_info.id,
    }
    .step_on(repo, context)
    .await?;

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

    let counters = ApplyUnitEdits {
        page_id: &page_info.id,
        orders: &orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    SetPageUnitCounters {
        id: &page_info.id,
        counters,
    }
    .step_on(repo, context)
    .await?;

    let old_counters = UnitCountMetrics {
        total: page_info.total_unit_count,
        translated: page_info.translated_unit_count,
        proofread: page_info.proofread_unit_count,
    };

    let counter_delta = old_counters.calc_delta(counters)?;

    accept(Some((counter_delta, edits)))
}

// Applies Chapter aggregates and workflow effects after Unit edits.
async fn finish_transform<C, R>(
    repo: &R,
    context: &mut C,
    chapter_info: &ChapterInfo,
    token: &UserToken,
    total_delta: UnitCountDelta,
    applied_edits: &[UnitEdit],
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
    if applied_edits.is_empty() {
        return accept(());
    }

    AdjustChapterUnitCounters {
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

    let stages = UnitComplex::submitted_stage_advances(applied_edits);

    start_pending_stages(
        repo,
        context,
        &chapter_info.id,
        Some(token.user_id.clone()),
        ChapterWorkflowRecordOrigin::UnitEdit,
        &stages,
    )
    .await?;

    accept(())
}
