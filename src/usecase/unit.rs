//! Unit use cases for listing and saving one Page sequence.

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::unit::{
    ListPageUnitInfosParams, ListPageUnitInfosPayload, SavePageUnitEditsParams,
    into_unit_edits,
};
use crate::model::read::proj::unit::UnitCounters;
use crate::model::user::UserToken;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, GetChapterInfo, GetChapterInfoExcluded,
};
use crate::part::repo::oper::comic::{GetComicInfo, TouchComicLastActive};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, SetPageUnitCounters,
};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitOrders,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, accept};
use crate::usecase::stage::spawn_starts;
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

#[cfg(test)]
mod tests;

#[instrument(level = "info", err(Debug), skip(repo))]
/// Lists visible Units for one Page in final linked-list order.
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: ListPageUnitInfosParams,
) -> BaseResult<ListPageUnitInfosPayload>
where
    R: PageRepo<C>
        + UnitRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
{
    let page_info = repo
        .run(&GetPageInfo {
            id: &params.page_id,
        })
        .await?;

    UnitPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    let unit_infos = repo
        .run(&ListUnitInfos {
            page_id: &page_info.id,
        })
        .await?;

    let counters = UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    };

    accept(ListPageUnitInfosPayload::from_parts(unit_infos, counters))
}

#[instrument(level = "info", err(Debug), skip(nucl, repo))]
/// Saves one authorized batch of Unit edits without returning a payload.
pub async fn save_edits<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: SavePageUnitEditsParams,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: PageRepo<C>
        + UnitRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + Clone
        + Send
        + Sync
        + 'static,
{
    let SavePageUnitEditsParams { page_id, edits } = params;

    let edits = into_unit_edits(edits, &token.user_id, UnitComplex::gen_id)?;

    let stages = UnitComplex::submitted_stage_starts(&edits);

    let page_scope = repo.run(&GetPageInfo { id: &page_id }).await?;

    let chapter_id = nucl
        .coord(async move |context| {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &page_scope.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let page_info = repo
                .step(context, &GetPageInfoExcluded { id: &page_id })
                .await?;

            let assignment = repo
                .step(
                    context,
                    &FindAssignmentInfo::ChapterUser {
                        chapter_id: &page_info.chapter_id,
                        user_id: &token.user_id,
                    },
                )
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

            let orders = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: &page_info.id,
                    },
                )
                .await?;

            let base_ids = orders
                .iter()
                .map(|order| order.id.as_str())
                .collect::<Vec<_>>();

            let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

            let counters = repo
                .step(
                    context,
                    &ApplyUnitEdits {
                        page_id: &page_info.id,
                        orders: &orders,
                        edits: &edits,
                    },
                )
                .await?;

            repo.step(
                context,
                &SetPageUnitCounters {
                    id: &page_info.id,
                    counters,
                },
            )
            .await?;

            let old_counters = UnitCounters {
                total_unit_count: page_info.total_unit_count,
                translated_unit_count: page_info.translated_unit_count,
                proofread_unit_count: page_info.proofread_unit_count,
            };

            let delta = old_counters.calc_delta(counters);

            repo.step(
                context,
                &AdjustChapterUnitCounters {
                    id: &page_info.chapter_id,
                    delta,
                },
            )
            .await?;

            repo.step(
                context,
                &TouchComicLastActive {
                    id: &chapter_info.comic_id,
                },
            )
            .await?;

            accept(page_info.chapter_id)
        })
        .await?;

    spawn_starts(((*repo).clone(),), chapter_id, stages);

    accept(())
}
