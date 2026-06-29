//! Unit use cases — list and save page unit sequences.

use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::unit::{
    ListPageUnitInfosData, ListPageUnitInfosVal, SavePageUnitsData, SavePageUnitsVal, UnitInfoVal,
};
use crate::model::unit::{UnitCounterDelta, UnitCounters};
use crate::model::user::UserToken;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::unit::UnitStep;
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists units under one page.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListPageUnitInfosData,
) -> RootResult<ListPageUnitInfosVal>
where
    R: PageRepo<C>
        + UnitRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    <R as DeriveTransactional>::Transactional: PageRepoTransactional<C>
        + UnitRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>,
{
    let page_info = repo
        .execute(&PageStep::get_info_by_id(&data.page_id))
        .await?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        UnitPermComplex::can_user_list_infos(
            &mut repo.as_proxy(),
            &token.user_id,
            &page_info.chapter_id,
        )
        .await?;
    }

    let unit_infos = repo
        .execute(&UnitStep::list_infos_by_page(&page_info.id))
        .await?;

    accept(ListPageUnitInfosVal {
        units: unit_infos.into_iter().map(UnitInfoVal::from).collect(),
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    })
}

/// Saves ordered unit operations under one page.
pub async fn save_infos<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: SavePageUnitsData,
) -> RootResult<SavePageUnitsVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: PageRepo<C> + UnitRepo<C> + ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: PageRepoTransactional<C>
        + UnitRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    let opers = data.opers;

    let save_result = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            // FIXME: Add page-scoped SubmissionId deduplication before operation replay.
            let page_info = repo
                .advance(context, &PageStep::get_info_excluded(&data.page_id))
                .await?;

            {
                use crate::part::shared::proxy::AsProxyTransactional as _;

                UnitPermComplex::can_user_save_infos(
                    &mut repo.as_proxy(context),
                    &token.user_id,
                    &page_info.chapter_id,
                )
                .await?;
            }

            let chapter_info = repo
                .advance(context, &ChapterStep::get_info_by_id(&page_info.chapter_id))
                .await?;

            let current_unit_infos = repo
                .advance(context, &UnitStep::list_infos_by_page(&page_info.id))
                .await?;

            let unit_opers = opers.into_iter().map(Into::into).collect::<Vec<_>>();
            let now = OffsetDateTime::now_utc();

            let applied =
                UnitComplex::apply_opers(&page_info.id, current_unit_infos, unit_opers, now)?;

            repo.advance(
                context,
                &UnitStep::replace_infos_by_page(&page_info.id, &applied.unit_infos),
            )
            .await?;

            let counters = repo
                .advance(context, &UnitStep::count_by_page(&page_info.id))
                .await?;

            repo.advance(
                context,
                &PageStep::set_unit_counters(&page_info.id, counters),
            )
            .await?;

            let old_counters = UnitCounters {
                total_unit_count: page_info.total_unit_count,
                translated_unit_count: page_info.translated_unit_count,
                proofread_unit_count: page_info.proofread_unit_count,
            };

            let delta = counter_delta(old_counters, counters);

            repo.advance(
                context,
                &ChapterStep::adjust_unit_counters(&page_info.chapter_id, delta),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            accept(SavePageUnitsVal::from_parts(
                applied.unit_infos,
                applied.id_mapper,
                counters,
            ))
        })
        .await
        .map_err(map_drive_err)?;

    accept(save_result)
}

fn counter_delta(old_counters: UnitCounters, new_counters: UnitCounters) -> UnitCounterDelta {
    UnitCounterDelta {
        total_unit_count: new_counters.total_unit_count - old_counters.total_unit_count,
        translated_unit_count: new_counters.translated_unit_count
            - old_counters.translated_unit_count,
        proofread_unit_count: new_counters.proofread_unit_count - old_counters.proofread_unit_count,
    }
}
