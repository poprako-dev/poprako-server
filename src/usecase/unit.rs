//! Unit use cases — list and save page unit sequences.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::unit::{
    ListPageUnitInfosData, ListPageUnitInfosVal, SavePageUnitsData,
    SavePageUnitsVal, UnitInfoVal,
};
use crate::model::unit::{
    UnitApplyAck, UnitCounterDelta, UnitCounters, UnitIdMapper, UnitOper,
};
use crate::model::user::UserToken;
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::unit::UnitStep;
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists units under one page.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListPageUnitInfosData,
) -> RegularResult<ListPageUnitInfosVal>
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

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    UnitPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    let unit_infos = repo
        .execute(&UnitStep::list_infos_by_page_id(
            &page_info.id,
            Page {
                offset: data.offset,
                limit: data.limit,
            },
        ))
        .await?;

    Ok(ListPageUnitInfosVal {
        unit_infos: unit_infos.into_iter().map(UnitInfoVal::from).collect(),
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    })
}

/// Saves unit opers under one page.
pub async fn save_infos<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: SavePageUnitsData,
) -> RegularResult<SavePageUnitsVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: PageRepo<C>
        + UnitRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: PageRepoTransactional<C>
        + UnitRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    let SavePageUnitsData { page_id, diff } = data;

    if diff.page_id != page_id {
        return Err(unit_invalid_oper_error());
    }

    let unit_diff = diff.into_model().ok_or_else(unit_invalid_oper_error)?;

    let UnitApplyParts {
        opers,
        local_id_maps,
    } = UnitApplyParts::from(UnitComplex::prepare_diff(unit_diff)?);

    let save_units = drive
        .with_context(async move |context| -> RegularResult<SavePageUnitsVal> {
            //
            let repo = repo.derive_transactional().await;

            let page_info = repo
                .advance(context, &PageStep::get_info_excluded(&page_id))
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
                .advance(
                    context,
                    &ChapterStep::get_info_by_id(&page_info.chapter_id, &[]),
                )
                .await?;

            let current_indexes = repo
                .advance(
                    context,
                    &UnitStep::list_indexes_by_page_id(&page_info.id),
                )
                .await?;

            let mut sorted_indexes = current_indexes.clone();

            sorted_indexes.sort_by(|left, right| {
                left.index
                    .cmp(&right.index)
                    .then_with(|| left.id.cmp(&right.id))
            });

            let mut current_order: Vec<String> = sorted_indexes
                .into_iter()
                .map(|unit_index| unit_index.id)
                .collect();

            for oper in &opers {
                match oper {
                    //
                    UnitOper::Save { .. } => {
                        repo.advance(
                            context,
                            &UnitStep::save_info(&page_info.id, oper),
                        )
                        .await?;
                    }

                    UnitOper::Delete { id } => {
                        repo.advance(
                            context,
                            &UnitStep::delete_by_id_in_page(&page_info.id, id),
                        )
                        .await?;
                    }
                }
            }

            current_order =
                UnitComplex::apply_opers_to_order(&opers, current_order);

            let index_updates = UnitComplex::build_index_updates_from_order(
                &current_order,
                &current_indexes,
            );

            if !index_updates.is_empty() {
                repo.advance(
                    context,
                    &UnitStep::update_indexes_by_page_id(
                        &page_info.id,
                        &index_updates,
                    ),
                )
                .await?;
            }

            let counters = repo
                .advance(context, &UnitStep::count_by_page_id(&page_info.id))
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
                &ChapterStep::adjust_unit_counters(
                    &page_info.chapter_id,
                    delta,
                ),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            Ok(SavePageUnitsVal::from_parts(local_id_maps, counters))
        })
        .await?;

    Ok(save_units)
}

/// Carries the prepared opers and local ID maps produced by applying a diff.
struct UnitApplyParts {
    opers: Vec<UnitOper>,
    local_id_maps: Vec<UnitIdMapper>,
}

impl From<UnitApplyAck> for UnitApplyParts {
    fn from(receipt: UnitApplyAck) -> Self {
        Self {
            opers: receipt.opers,
            local_id_maps: receipt.local_id_map,
        }
    }
}

/// Computes the per-counter delta between old and new unit counters.
fn counter_delta(
    old_counters: UnitCounters,
    new_counters: UnitCounters,
) -> UnitCounterDelta {
    UnitCounterDelta {
        total_unit_count: new_counters.total_unit_count
            - old_counters.total_unit_count,
        translated_unit_count: new_counters.translated_unit_count
            - old_counters.translated_unit_count,
        proofread_unit_count: new_counters.proofread_unit_count
            - old_counters.proofread_unit_count,
    }
}

/// Constructs an args error for an invalid unit operation.
fn unit_invalid_oper_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}
