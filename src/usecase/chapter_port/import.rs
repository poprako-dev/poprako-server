use std::collections::HashSet;

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::chapter_port::{
    ChapterImportComplex, ChapterPortPermComplex,
};
use crate::complex::unit::{UnitComplex, UnitPermComplex};
use crate::data::chapter_port::{
    ImportChapterTranslationParams, ImportChapterTranslationPayload,
};
use crate::model::page::PageInfo;
use crate::model::read::proj::unit::{
    UnitCounterDelta, UnitCounters, UnitOrder,
};
use crate::model::unit_port::UnitTranslationImport;
use crate::model::user::UserToken;
use crate::model::write::unit::UnitEdit;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, GetChapterInfoExcluded,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, ListPageInfos, SetPageUnitCounters,
};
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::usecase::stage::spawn_starts;
use crate::util::PatchField;
use crate::value::chapter::Stage;
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

#[cfg(test)]
mod tests;

fn visible_unit_ids(unit_orders: &[UnitOrder]) -> Vec<String> {
    unit_orders
        .iter()
        .filter(|unit_order| !unit_order.is_hidden)
        .map(|unit_order| unit_order.id.clone())
        .collect()
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
/// Imports chapter translation content through the Unit edit pipeline.
pub async fn import<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: ImportChapterTranslationParams,
    chapter_id: String,
) -> BaseResult<ImportChapterTranslationPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Clone
        + Send
        + Sync
        + 'static,
{
    ChapterPortPermComplex::ensure_user_can_import(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    let assignment_info = repo
        .run(&FindAssignmentInfo::ChapterUser {
            chapter_id: &chapter_id,
            user_id: &token.user_id,
        })
        .await?
        .ok_or_else(unit_edit_permission_err)?;

    let edit_perm = UnitEditPerm {
        can_translate: assignment_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR]),
        can_proofread: assignment_info
            .roles
            .has_any_role(&[RoleField::PROOFREADER]),
    };

    let label_plus = matches!(params.format, TranslationFormat::LabelPlus);

    let imported_pages = match params.format {
        //
        TranslationFormat::LabelPlus => {
            ChapterImportComplex::parse_label_plus(&params.content)?
        }

        TranslationFormat::PopRaKo => {
            ChapterImportComplex::parse_poprako(&params.content)?
        }
    };

    let stage_chapter_id = chapter_id.clone();

    let import_payload = nucl
        .coord(async move |context| {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let page_scopes = repo
                .step(
                    context,
                    &ListPageInfos {
                        chapter_id: &chapter_id,
                    },
                )
                .await?;

            ChapterImportComplex::validate_page_count(
                imported_pages.len(),
                page_scopes.len(),
            )?;

            let mut imported_unit_count = 0;

            for (page_scope, imported_page) in
                page_scopes.iter().zip(imported_pages.iter())
            {
                let page_info = repo
                    .step(context, &GetPageInfoExcluded { id: &page_scope.id })
                    .await?;

                let mut positions = repo
                    .step(
                        context,
                        &ListUnitOrders {
                            page_id: &page_info.id,
                        },
                    )
                    .await?;

                if imported_page.units.is_empty() {
                    continue;
                }

                let edits = build_page_edits(
                    &imported_page.units,
                    &positions,
                    &token.user_id,
                    edit_perm,
                    label_plus,
                )?;

                UnitPermComplex::ensure_user_can_edit_fields(
                    edit_perm, &edits,
                )?;

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

                let delta = counter_delta(page_counters(&page_info), counters);

                repo.step(
                    context,
                    &AdjustChapterUnitCounters {
                        id: &page_info.chapter_id,
                        delta,
                    },
                )
                .await?;

                imported_unit_count += imported_page.units.len() as i32;
            }

            repo.step(
                context,
                &TouchComicLastActive {
                    id: &chapter_info.comic_id,
                },
            )
            .await?;

            accept(ImportChapterTranslationPayload {
                imported_page_count: page_scopes.len() as i32,
                imported_unit_count,
            })
        })
        .await?;

    let stages = import_stages(edit_perm);

    spawn_starts(((*repo).clone(),), stage_chapter_id, stages);

    accept(import_payload)
}

fn build_page_edits(
    imported_units: &[UnitTranslationImport],
    positions: &[UnitOrder],
    user_id: &str,
    edit_perm: UnitEditPerm,
    label_plus: bool,
) -> BaseResult<Vec<UnitEdit>> {
    //
    let visible_ids = visible_unit_ids(positions);

    let known_ids = positions
        .iter()
        .map(|position| position.id.as_str())
        .collect::<HashSet<_>>();

    let mut edits = Vec::with_capacity(imported_units.len());

    for imported_unit in imported_units {
        //
        let candidate_id = imported_unit
            .id
            .as_deref()
            .filter(|id| known_ids.contains(*id))
            .map(str::to_string)
            .or_else(|| {
                usize::try_from(imported_unit.index)
                    .ok()
                    .and_then(|index| visible_ids.get(index).cloned())
            });

        let (unit_id, next_id) = match candidate_id {
            //
            Some(unit_id) => (unit_id, PatchField::Skip),

            None => {
                //
                let unit_id = UnitComplex::gen_id();

                (unit_id, PatchField::Clear)
            }
        };

        let edit = ChapterImportComplex::build_unit_edit(
            imported_unit,
            unit_id,
            next_id,
            user_id,
            edit_perm.can_translate,
            edit_perm.can_proofread,
            label_plus,
        );

        edits.push(edit);
    }

    accept(edits)
}

fn page_counters(page_info: &PageInfo) -> UnitCounters {
    UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    }
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

fn import_stages(edit_perm: UnitEditPerm) -> Vec<Stage> {
    //
    let mut stages = Vec::with_capacity(2);

    if edit_perm.can_translate {
        stages.push(Stage::Translate);
    }

    if edit_perm.can_proofread {
        stages.push(Stage::Proofread);
    }

    stages
}

fn unit_edit_permission_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}
