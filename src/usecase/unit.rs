//! Unit use cases for listing, searching, saving, and transforming text.

/// Chapter-scoped Unit text transformation.
pub mod transform;

#[cfg(test)]
// Unit tests for unit creation, editing, and transition rules.
mod tests;

use std::collections::HashMap;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl_kv;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::UnitComplex;
use crate::complex::unit::perm::UnitPermComplex;
use crate::data::instr::unit::{
    ListPageUnitInfosInstr, SavePageUnitEditsInstr,
    SearchChapterUnitInfosInstr, UnitEditInstr, into_unit_edits,
};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::data::view::unit::UnitInfoView;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitInfo, UnitOrder};
use crate::model::shared::user::UserToken;
use crate::part::nucl::{ReptRead, Serial};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCountDelta, GetChapterUnitEditScopeExcluded,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    GetPageUnitScope, GetPageUnitScopeExcluded, SetPageUnitCountMetrics,
};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitInfosInChapterOrder,
    ListUnitOrders, SearchChapterUnitIds,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::unit::UnitAccessLoader;
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::role::RoleField;
use crate::value::unit::{
    MAX_PAGE_UNIT_COUNT, MAX_UNIT_SEARCH_MATCH_COUNT, UnitEditPerm,
};

// Fixed-size diagnostics for a potentially large Unit edit request.
#[derive(Debug, Default)]
struct UnitEditLogSummary {
    //
    // Number of create operations.
    creates: usize,
    // Number of patch operations.
    patches: usize,
    // Number of delete operations.
    deletes: usize,
}

impl UnitEditLogSummary {
    // Builds fixed-size diagnostics without retaining edit payloads.
    fn from_edits(edits: &[UnitEditInstr]) -> Self {
        //
        let mut summary = Self::default();

        for edit in edits {
            //
            match edit {
                //
                UnitEditInstr::Create { .. } => summary.creates += 1,

                UnitEditInstr::Patch { .. } => summary.patches += 1,

                UnitEditInstr::Delete { .. } => summary.deletes += 1,
            }
        }

        summary
    }
}

/// Lists visible Units for one Page in final linked-list order.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: ListPageUnitInfosInstr,
) -> BaseRest<ListPageUnitInfosVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: PageRepo<C>
        + UnitRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    let page_scope =
        GetPageUnitScope { id: &instr.page_id }.run_on(repo).await?;

    let access_info = UnitAccessLoader::load_access_info_from_chapter::<C, R>(
        repo,
        &token.user_id,
        &page_scope.chapter_id,
    )
    .await?;

    UnitPermComplex::ensure_user_can_list_infos(&access_info.as_access())?;

    let authorized_chapter_id = page_scope.chapter_id;

    let (unit_infos, count_metrics) = nucl
        .coord(async move |context| {
            //
            let page_scope = GetPageUnitScope { id: &instr.page_id }
                .step_on(repo, context)
                .await?;

            if page_scope.chapter_id != authorized_chapter_id {
                //
                return Err(unit_list_invariant(
                    &page_scope.id,
                    "Page ownership changed during Unit list authorization",
                ));
            }

            let orders = ListUnitOrders {
                page_id: &page_scope.id,
            }
            .step_on(repo, context)
            .await?;

            let unit_infos =
                load_visible_unit_infos(repo, context, &page_scope.id, &orders)
                    .await?;

            let actual_count_metrics = count_unit_infos(&unit_infos);

            if actual_count_metrics != page_scope.count_metrics {
                //
                return Err(unit_list_invariant(
                    &page_scope.id,
                    "Page Unit counters do not match visible Units",
                ));
            }

            accept((unit_infos, page_scope.count_metrics))
        })
        .await?;

    accept(ListPageUnitInfosVal::from_parts(unit_infos, count_metrics))
}

#[instrument(
    level = "info",
    skip(nucl, repo, token),
    fields(
        actor_user_id = %token.user_id,
        chapter_id = %instr.chapter_id,
        part = ?instr.part,
    ),
)]
/// Searches one Unit text field across all visible Units in a Chapter.
pub async fn search_infos<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: SearchChapterUnitInfosInstr,
) -> BaseRest<Vec<UnitInfoView>>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: UnitRepo<C> + TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
{
    let SearchChapterUnitInfosInstr {
        chapter_id,
        part,
        phrase,
    } = instr;

    let phrase = UnitComplex::normalize_search_phrase(phrase)?;

    let access_info = UnitAccessLoader::load_access_info_from_chapter::<C, R>(
        repo,
        &token.user_id,
        &chapter_id,
    )
    .await?;

    UnitPermComplex::ensure_user_can_list_infos(&access_info.as_access())?;

    if phrase.contains('\0') {
        return accept(Vec::new());
    }

    let found_infos = nucl
        .coord(async move |context| {
            //
            let search_ids = SearchChapterUnitIds {
                chapter_id: &chapter_id,
                part,
                phrase: &phrase,
                fetch_count: MAX_UNIT_SEARCH_MATCH_COUNT + 1,
            }
            .step_on(repo, context)
            .await?;

            if search_ids.len() > MAX_UNIT_SEARCH_MATCH_COUNT {
                //
                let args = HashMap::from([(
                    "match_limit".into(),
                    MAX_UNIT_SEARCH_MATCH_COUNT.into(),
                )]);

                let err_message =
                    trl_kv("error-unit-search-too-many-matches", &args);

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    match_count = search_ids.len(),
                    match_limit = MAX_UNIT_SEARCH_MATCH_COUNT,
                    "expected error: too many Unit search matches",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let search_id_refs =
                search_ids.iter().map(String::as_str).collect::<Vec<_>>();

            let unit_infos = ListUnitInfosInChapterOrder {
                ids: &search_id_refs,
            }
            .step_on(repo, context)
            .await?;

            accept(unit_infos.into_iter().map(UnitInfoView::from).collect())
        })
        .await?;

    accept(found_infos)
}

/// Saves one authorized batch of Unit edits without returning a payload.
#[instrument(
    level = "info",
    skip(nucl, repo, token, instr),
    fields(
        actor_user_id = %token.user_id,
        page_id = %instr.page_id,
        edit_count = instr.edits.len(),
        edit_kinds = ?UnitEditLogSummary::from_edits(&instr.edits),
    ),
)]
pub async fn save_edits<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: SavePageUnitEditsInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: PageRepo<C>
        + UnitRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let SavePageUnitEditsInstr { page_id, edits } = instr;

    let edits = into_unit_edits(edits, &token.user_id, UnitComplex::gen_id)?;

    let stages = UnitComplex::submitted_stage_advances(&edits);

    let () = nucl
        .coord(async move |context| {
            //
            let chapter_scope =
                GetChapterUnitEditScopeExcluded { page_id: &page_id }
                    .step_on(repo, context)
                    .await?;

            ChapterComplex::ensure_writable_state(
                &chapter_scope.id,
                chapter_scope.is_published,
            )?;

            let assignment = FindAssignmentInfo::ChapterUser {
                chapter_id: &chapter_scope.id,
                user_id: &token.user_id,
            }
            .step_on(repo, context)
            .await?;

            let edit_perm = UnitEditPerm {
                can_translate: assignment.as_ref().is_some_and(|assignment| {
                    assignment.roles.has_any_role(&[RoleField::TRANSLATOR])
                }),
                can_proofread: assignment.as_ref().is_some_and(|assignment| {
                    assignment.roles.has_any_role(&[RoleField::PROOFREADER])
                }),
            };

            UnitPermComplex::ensure_user_can_edit_fields(edit_perm, &edits)?;

            let page_scope = GetPageUnitScopeExcluded { id: &page_id }
                .step_on(repo, context)
                .await?;

            if page_scope.chapter_id != chapter_scope.id {
                //
                return Err(BaseError::Unrecoverable {
                    message: "locked Page does not belong to locked Chapter"
                        .into(),
                });
            }

            let orders = ListUnitOrders {
                page_id: &page_scope.id,
            }
            .step_on(repo, context)
            .await?;

            let base_ids = orders
                .iter()
                .map(|order| order.id.as_str())
                .collect::<Vec<_>>();

            let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

            let count_metrics = ApplyUnitEdits {
                page_id: &page_scope.id,
                orders: &orders,
                edits: &edits,
            }
            .step_on(repo, context)
            .await?;

            SetPageUnitCountMetrics {
                id: &page_scope.id,
                count_metrics,
            }
            .step_on(repo, context)
            .await?;

            let delta = page_scope.count_metrics.calc_delta(count_metrics)?;

            AdjustChapterUnitCountDelta {
                id: &chapter_scope.id,
                delta,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_scope.comic_id,
            }
            .step_on(repo, context)
            .await?;

            start_pending_stages(
                repo,
                context,
                &chapter_scope.id,
                Some(token.user_id.clone()),
                ChapterWorkflowRecordOrigin::UnitEdit,
                &stages,
            )
            .await?;

            accept(())
        })
        .await?;

    accept(())
}

// Builds an unrecoverable error for an inconsistent persisted Unit list.
fn unit_list_invariant(page_id: &str, message: &'static str) -> BaseError {
    //
    tracing::error!(
        page_id,
        err_message = message,
        "unrecoverable error: invalid persisted Page Unit state",
    );

    BaseError::Unrecoverable {
        message: message.into(),
    }
}

// Loads and verifies every visible Unit in linked-list order.
async fn load_visible_unit_infos<C, R>(
    repo: &R,
    context: &mut C,
    page_id: &str,
    orders: &[UnitOrder],
) -> BaseRest<Vec<UnitInfo>>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    R: UnitRepo<C> + Sync,
{
    //
    let visible_orders = orders
        .iter()
        .filter(|order| !order.is_hidden)
        .collect::<Vec<_>>();

    if visible_orders.len() > MAX_PAGE_UNIT_COUNT {
        //
        return Err(unit_list_invariant(
            page_id,
            "persisted Page Unit count exceeds the business maximum",
        ));
    }

    let visible_ids = visible_orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let selected_infos = match visible_ids.as_slice() {
        //
        [] => Vec::new(),

        ids => ListUnitInfosByIds { ids }.step_on(repo, context).await?,
    };

    let expected_next_by_id = visible_orders
        .iter()
        .map(|order| (order.id.as_str(), order.next_id.as_deref()))
        .collect::<HashMap<_, _>>();

    if expected_next_by_id.len() != visible_orders.len() {
        //
        return Err(unit_list_invariant(
            page_id,
            "persisted Unit chain contains duplicate IDs",
        ));
    }

    let mut info_by_id = HashMap::with_capacity(selected_infos.len());

    for unit_info in selected_infos {
        //
        let Some(expected_next_id) =
            expected_next_by_id.get(unit_info.id.as_str()).copied()
        else {
            //
            return Err(unit_list_invariant(
                page_id,
                "Unit detail query returned an unexpected ID",
            ));
        };

        if unit_info.page_id != page_id
            || unit_info.hidden_at.is_some()
            || unit_info.next_id.as_deref() != expected_next_id
        {
            //
            return Err(unit_list_invariant(
                page_id,
                "Unit detail query disagrees with the verified chain",
            ));
        }

        let unit_id = unit_info.id.clone();

        if info_by_id.insert(unit_id, unit_info).is_some() {
            //
            return Err(unit_list_invariant(
                page_id,
                "Unit detail query returned duplicate rows",
            ));
        }
    }

    let mut ordered_infos = Vec::with_capacity(visible_orders.len());

    for order in visible_orders {
        //
        let Some(unit_info) = info_by_id.remove(&order.id) else {
            //
            return Err(unit_list_invariant(
                page_id,
                "Unit detail query omitted a visible Unit",
            ));
        };

        ordered_infos.push(unit_info);
    }

    accept(ordered_infos)
}

// Counts visible Units by completion state.
fn count_unit_infos(unit_infos: &[UnitInfo]) -> UnitCountMetrics {
    //
    UnitCountMetrics {
        total: unit_infos.len(),
        translated: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_translated())
            .count(),
        proofread: unit_infos
            .iter()
            .filter(|unit_info| unit_info.is_proofread)
            .count(),
    }
}
