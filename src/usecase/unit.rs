//! Unit use cases for listing and saving one Page sequence.

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::unit::{
    ListPageUnitInfosParams, ListPageUnitInfosPayload, SavePageUnitEditsParams,
    into_unit_edits,
};
use crate::model::read::proj::unit::{
    UnitCounterDelta, UnitCounters, UnitOrder,
};
use crate::model::user::UserToken;
use crate::model::write::unit::UnitEdit;
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
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::usecase::stage::spawn_starts;
use crate::util::PatchField;
use crate::value::chapter::Stage;
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

fn invalid_unit_edit_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-invalid-unit-oper"),
    }
}

fn move_unit_order(
    unit_orders: &mut Vec<UnitOrder>,
    id: &str,
    next_id: Option<&String>,
) -> BaseResult<()> {
    //
    let Some(position) = unit_orders
        .iter()
        .position(|unit_order| unit_order.id == id)
    else {
        return Err(invalid_unit_edit_err());
    };

    let unit_order = unit_orders.remove(position);

    let target = match next_id {
        //
        Some(next_id) => unit_orders
            .iter()
            .position(|candidate| candidate.id == *next_id)
            .ok_or_else(invalid_unit_edit_err)?,

        None => unit_orders.len(),
    };

    unit_orders.insert(target, unit_order);

    accept(())
}

fn relink_unit_orders(unit_orders: &mut [UnitOrder]) {
    for index in 0..unit_orders.len() {
        unit_orders[index].next_id = unit_orders
            .get(index + 1)
            .map(|unit_order| unit_order.id.clone());
    }
}

fn apply_unit_order_edits(
    unit_orders: &mut Vec<UnitOrder>,
    edits: &[UnitEdit],
) -> BaseResult<()> {
    //
    for edit in edits {
        match edit {
            //
            UnitEdit::Delete { id } => {
                //
                let Some(unit_order) = unit_orders
                    .iter_mut()
                    .find(|unit_order| unit_order.id == *id)
                else {
                    return Err(invalid_unit_edit_err());
                };

                unit_order.is_hidden = true;
            }

            UnitEdit::Save { id, next_id, .. } => {
                //
                let position = unit_orders
                    .iter()
                    .position(|unit_order| unit_order.id == *id);

                match position {
                    //
                    Some(position) => {
                        unit_orders[position].is_hidden = false;
                    }

                    None => unit_orders.push(UnitOrder {
                        id: id.clone(),
                        next_id: None,
                        is_hidden: false,
                    }),
                }

                match next_id {
                    //
                    PatchField::Skip => {}

                    PatchField::Clear => {
                        move_unit_order(unit_orders, id, None)?;
                    }

                    PatchField::Assign(next_id) => {
                        move_unit_order(unit_orders, id, Some(next_id))?;
                    }
                }
            }
        }
    }

    relink_unit_orders(unit_orders);

    let visible_count = unit_orders
        .iter()
        .filter(|unit_order| !unit_order.is_hidden)
        .count();

    if visible_count > 100 {
        return Err(invalid_unit_edit_err());
    }

    accept(())
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

    let stages = submitted_stage_starts(&edits);

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

            let mut positions = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: &page_info.id,
                    },
                )
                .await?;

            let base_ids = positions
                .iter()
                .map(|position| position.id.as_str())
                .collect::<Vec<_>>();

            let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

            apply_unit_order_edits(&mut positions, &edits)?;

            let counters = repo
                .step(
                    context,
                    &ApplyUnitEdits {
                        page_id: &page_info.id,
                        orders: &positions,
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

            accept(page_info.chapter_id)
        })
        .await?;

    spawn_starts(((*repo).clone(),), chapter_id, stages);

    accept(())
}

fn submitted_stage_starts(edits: &[UnitEdit]) -> Vec<Stage> {
    //
    let translated = edits.iter().any(|edit| {
        matches!(
            edit,
            UnitEdit::Save {
                translation: PatchField::Assign(translation),
                ..
            } if !translation.translated_text.trim().is_empty()
        )
    });

    let proofread = edits.iter().any(|edit| {
        matches!(
            edit,
            UnitEdit::Save {
                revision: PatchField::Assign(revision),
                ..
            } if revision.is_proofread
        )
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
