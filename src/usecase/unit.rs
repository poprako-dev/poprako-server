//! Unit use cases for listing, searching, saving, and transforming text.

/// Chapter-scoped Unit text transformation.
pub mod transform;

#[cfg(test)]
// Unit tests for unit creation, editing, and transition rules.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::instr::unit::{
    ListPageUnitInfosInstr, SavePageUnitEditsInstr,
    SearchChapterUnitInfosInstr, into_unit_edits,
};
use crate::data::val::unit::ListPageUnitInfosVal;
use crate::data::view::unit::UnitInfoView;
use crate::model::read::proj::unit::UnitCounters;
use crate::model::shared::user::UserToken;
use crate::part::nucl::Serial;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, GetChapterInfoExcluded,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, ListPageInfos, SetPageUnitCounters,
};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::unit::UnitAccessLoader;
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordOrigin;
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

// Search pages in bounded concurrent batches.
const SEARCH_PAGE_BATCH_SIZE: usize = 20;

#[instrument(level = "info", skip(repo))]
/// Lists visible Units for one Page in final linked-list order.
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListPageUnitInfosInstr,
) -> BaseRest<ListPageUnitInfosVal>
where
    C: Context,
    R: PageRepo<C>
        + UnitRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    let page_info = GetPageInfo { id: &instr.page_id }.run_on(repo).await?;

    let access_info = UnitAccessLoader::load_access_info_from_chapter::<C, R>(
        repo,
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    UnitPermComplex::ensure_user_can_list_infos(access_info.as_access())?;

    let unit_infos = ListUnitInfos {
        page_id: &page_info.id,
    }
    .run_on(repo)
    .await?;

    let counters = UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    };

    accept(ListPageUnitInfosVal::from_parts(unit_infos, counters))
}

#[instrument(
    level = "info",
    skip(repo, token, instr),
    fields(chapter_id = %instr.chapter_id, part = ?instr.part),
)]
/// Searches one Unit text field across all visible Units in a Chapter.
pub async fn search_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: SearchChapterUnitInfosInstr,
) -> BaseRest<Vec<UnitInfoView>>
where
    C: Context,
    R: PageRepo<C>
        + UnitRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
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

    UnitPermComplex::ensure_user_can_list_infos(access_info.as_access())?;

    let page_infos = ListPageInfos {
        chapter_id: &chapter_id,
    }
    .run_on(repo)
    .await?;

    let phrase = &phrase;

    let batches = page_infos.chunks(SEARCH_PAGE_BATCH_SIZE).map(
        |page_batch| async move {
            //
            let mut found_infos = Vec::new();

            for page_info in page_batch {
                //
                let unit_infos = ListUnitInfos {
                    page_id: &page_info.id,
                }
                .run_on(repo)
                .await?;

                found_infos.extend(
                    unit_infos
                        .into_iter()
                        .filter(|unit_info| unit_info.hidden_at.is_none())
                        .enumerate()
                        .filter_map(|(unit_index, unit_info)| {
                            //
                            UnitComplex::text_part_contains(
                                &unit_info, part, phrase,
                            )
                            .then_some((
                                page_info.index,
                                unit_index,
                                unit_info,
                            ))
                        }),
                );
            }

            accept(found_infos)
        },
    );

    let mut found_infos = futures_util::future::try_join_all(batches)
        .await?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    found_infos
        .sort_by_key(|(page_index, unit_index, _)| (*page_index, *unit_index));

    accept(
        found_infos
            .into_iter()
            .map(|(_, _, unit_info)| UnitInfoView::from(unit_info))
            .collect(),
    )
}

#[instrument(level = "info", skip(nucl, repo))]
/// Saves one authorized batch of Unit edits without returning a payload.
pub async fn save_edits<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: SavePageUnitEditsInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
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

    let page_scope = GetPageInfo { id: &page_id }.run_on(repo).await?;

    let () = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &page_scope.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let page_info = GetPageInfoExcluded { id: &page_id }
                .step_on(repo, context)
                .await?;

            let assignment = FindAssignmentInfo::ChapterUser {
                chapter_id: &page_info.chapter_id,
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

            let old_counters = UnitCounters {
                total_unit_count: page_info.total_unit_count,
                translated_unit_count: page_info.translated_unit_count,
                proofread_unit_count: page_info.proofread_unit_count,
            };

            let delta = old_counters.calc_delta(counters);

            AdjustChapterUnitCounters {
                id: &page_info.chapter_id,
                delta,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            start_pending_stages(
                repo,
                context,
                &page_info.chapter_id,
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
