use std::collections::HashMap;

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter_port::{
    ChapterImportComplex, ChapterPortPermComplex,
};
use crate::complex::unit::UnitComplex;
use crate::data::chapter_port::{
    ImportChapterTranslationParams, ImportChapterTranslationPayload,
};
use crate::model::assignment::AssignmentInfo;
use crate::model::page::PageInfo;
use crate::model::unit::{UnitCounterDelta, UnitCounters, UnitInfo};
use crate::model::unit_port::UnitTranslationImport;
use crate::model::user::UserToken;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, GetChapterInfo,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{ListPageInfos, SetPageUnitCounters};
use crate::part::repo::oper::unit::{
    CountUnits, ListUnitIndexes, ListUnitInfos, SaveUnit, UpdateUnitIndexes,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::RoleField;

#[cfg(test)]
mod tests;

/// Imports chapter translation text into existing pages.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn import<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: ImportChapterTranslationParams,
    chapter_id: String,
) -> RegularResult<ImportChapterTranslationPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
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
        .ok_or_else(unit_edit_permission_error)?;

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

    let imported =
        nucl
            .coord(
                async move |context| -> RegularResult<
                    ImportChapterTranslationPayload,
                > {
                    //
                    let chapter_info = repo
                        .step(
                            context,
                            &GetChapterInfo {
                                id: &chapter_id,
                                incls: &[],
                            },
                        )
                        .await?;

                    let page_infos = repo
                        .step(
                            context,
                            &ListPageInfos::AllChapter {
                                chapter_id: &chapter_id,
                            },
                        )
                        .await?;

                    ChapterImportComplex::validate_page_count(
                        imported_pages.len(),
                        page_infos.len(),
                    )?;

                    let mut imported_unit_count = 0;

                    for (page_info, imported_page) in
                        page_infos.iter().zip(imported_pages.iter())
                    {
                        let old_counters = page_counters(page_info);

                        let existing_unit_infos = repo
                            .step(
                                context,
                                &ListUnitInfos::AllPage {
                                    page_id: &page_info.id,
                                },
                            )
                            .await?;

                        let existing_by_id = existing_unit_infos
                            .iter()
                            .map(|unit_info| (unit_info.id.as_str(), unit_info))
                            .collect::<HashMap<_, _>>();

                        let existing_by_index = existing_unit_infos
                            .iter()
                            .map(|unit_info| (unit_info.index, unit_info))
                            .collect::<HashMap<_, _>>();

                        for imported_unit in &imported_page.units {
                            //
                            let unit_id = resolve_unit_id(
                                imported_unit,
                                &existing_by_index,
                            );

                            let existing_unit = existing_by_id
                                .get(unit_id.as_str())
                                .copied()
                                .or_else(|| {
                                    existing_by_index
                                        .get(&imported_unit.index)
                                        .copied()
                                });

                            let unit_payload =
                                ChapterImportComplex::build_unit_payload(
                                    imported_unit,
                                    existing_unit,
                                    &token.user_id,
                                    has_proofreader_role(&assignment_info),
                                    label_plus,
                                );

                            repo.step(
                                context,
                                &SaveUnit {
                                    page_id: &page_info.id,
                                    id: &unit_id,
                                    payload: &unit_payload,
                                },
                            )
                            .await?;
                        }

                        let current_indexes = repo
                            .step(
                                context,
                                &ListUnitIndexes {
                                    page_id: &page_info.id,
                                },
                            )
                            .await?;

                        let index_updates =
                            UnitComplex::build_index_updates(current_indexes);

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

                        let delta = counter_delta(old_counters, counters);

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

                    Ok(ImportChapterTranslationPayload {
                        imported_page_count: page_infos.len() as i32,
                        imported_unit_count,
                    })
                },
            )
            .await?;

    Ok(imported)
}

/// Extracts unit counters from a [`PageInfo`].
fn page_counters(page_info: &PageInfo) -> UnitCounters {
    UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    }
}

/// Resolves the unit ID from an import — uses the provided ID, falls back to
/// an existing unit with the same index, or generates a new one.
fn resolve_unit_id(
    imported_unit: &UnitTranslationImport,
    existing_by_index: &HashMap<i32, &UnitInfo>,
) -> String {
    //
    if let Some(id) = imported_unit
        .id
        .as_deref()
        .filter(|id| !id.trim().is_empty())
    {
        return id.trim().into();
    }

    if let Some(unit_info) = existing_by_index.get(&imported_unit.index) {
        return unit_info.id.clone();
    }

    UnitComplex::gen_id()
}

/// Returns true if the assignment grants a PROOFREADER role.
fn has_proofreader_role(assignment_info: &AssignmentInfo) -> bool {
    assignment_info
        .roles
        .has_any_role(&[RoleField::PROOFREADER])
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

/// Constructs a permission error for missing unit edit access.
fn unit_edit_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}
