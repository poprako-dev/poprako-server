//! Unit use cases — list and save page unit sequences.

use poprako_orchestra::{Nucl, run_proxy, step_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::unit::{
    ListPageUnitInfosParams, ListPageUnitInfosPayload, SavePageUnitsParams,
    SavePageUnitsPayload, UnitInfoVal,
};
use crate::model::unit::{
    UnitApplyAck, UnitCounterDelta, UnitCounters, UnitIdMapper, UnitOper,
};
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
    CountUnits, CreateUnit, DeleteUnit, ListUnitIndexes, ListUnitInfos,
    SaveUnit, UpdateUnitIndexes,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::usecase::stage::spawn_starts;
use crate::value::chapter::Stage;

#[cfg(test)]
mod tests;

/// Maximum number of units allowed on a single page.
const MAX_UNITS_PER_PAGE: usize = 100;

/// Lists all units under one page.
#[instrument(level = "info", err(Debug), skip_all)]
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

    accept(ListPageUnitInfosPayload {
        unit_infos: unit_infos.into_iter().map(UnitInfoVal::from).collect(),
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    })
}

/// Saves unit opers under one page.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn save<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: SavePageUnitsParams,
) -> BaseResult<SavePageUnitsPayload>
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
    let SavePageUnitsParams { page_id, diff } = params;

    if diff.page_id != page_id {
        return Err(unit_invalid_oper_err());
    }

    let unit_diff = diff.into_model().ok_or_else(unit_invalid_oper_err)?;

    let UnitApplyParts {
        opers,
        local_id_maps,
    } = UnitApplyParts::from(UnitComplex::prepare_diff(unit_diff)?);

    // A single unit save batch must not exceed 100 operations.
    if !(1..=100).contains(&opers.len()) {
        return Err(unit_invalid_oper_err());
    }

    let stages = submitted_stage_starts(&opers);

    let page_scope = repo.run(&GetPageInfo { id: &page_id }).await?;

    let net_create_count = opers
        .iter()
        .filter(|oper| matches!(oper, UnitOper::Create { .. }))
        .count()
        - opers
            .iter()
            .filter(|oper| matches!(oper, UnitOper::Delete { .. }))
            .count();

    let resulting_count =
        page_scope.total_unit_count as usize + net_create_count;

    if resulting_count > MAX_UNITS_PER_PAGE {
        return Err(unit_invalid_oper_err());
    }

    let saved_units = nucl
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

            UnitPermComplex::ensure_user_can_save_infos(
                &mut step_proxy! {
                    context;
                    repo =>
                        for<'a, 'b> FindAssignmentInfo<'a, 'b>;
                },
                &token.user_id,
                &page_info.chapter_id,
            )
            .await?;

            let current_indexes = repo
                .step(
                    context,
                    &ListUnitIndexes {
                        page_id: &page_info.id,
                    },
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
                    UnitOper::Create { id, payload, .. } => {
                        repo.step(
                            context,
                            &CreateUnit {
                                page_id: &page_info.id,
                                id,
                                payload,
                            },
                        )
                        .await?;
                    }

                    UnitOper::Save { id, payload, .. } => {
                        repo.step(
                            context,
                            &SaveUnit {
                                page_id: &page_info.id,
                                id,
                                payload,
                            },
                        )
                        .await?;
                    }

                    UnitOper::Delete { id } => {
                        repo.step(
                            context,
                            &DeleteUnit {
                                page_id: &page_info.id,
                                id,
                            },
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
                repo.step(
                    context,
                    &UpdateUnitIndexes {
                        page_id: &page_info.id,
                        updates: &index_updates,
                    },
                )
                .await?;
            }

            let counters = repo
                .step(
                    context,
                    &CountUnits {
                        page_id: &page_info.id,
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

            let delta = counter_delta(old_counters, counters);

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

            accept(SavePageUnitsResult {
                payload: SavePageUnitsPayload::from_parts(
                    local_id_maps,
                    counters,
                ),
                chapter_id: page_info.chapter_id,
            })
        })
        .await?;

    spawn_starts(((*repo).clone(),), saved_units.chapter_id, stages);

    accept(saved_units.payload)
}

struct SavePageUnitsResult {
    //
    payload: SavePageUnitsPayload,
    chapter_id: String,
}

/// Carries the validated opers and local ID maps produced by applying a diff.
struct UnitApplyParts {
    //
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

fn submitted_stage_starts(opers: &[UnitOper]) -> Vec<Stage> {
    //
    let translated = opers.iter().any(|oper| match oper {
        //
        UnitOper::Create { payload, .. } | UnitOper::Save { payload, .. } => {
            has_text(&payload.translated_text)
                && has_text(&payload.last_translator_id)
        }

        UnitOper::Delete { .. } => false,
    });

    let proofread = opers.iter().any(|oper| match oper {
        //
        UnitOper::Create { payload, .. } | UnitOper::Save { payload, .. } => {
            payload.is_proofread && has_text(&payload.last_proofreader_id)
        }

        UnitOper::Delete { .. } => false,
    });

    let mut stages = Vec::with_capacity(2);

    if translated {
        stages.push(Stage::Translate);
    }

    if proofread {
        stages.push(Stage::Proofread);
    }

    stages
}

fn has_text(text: &Option<String>) -> bool {
    text.as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
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
fn unit_invalid_oper_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}
