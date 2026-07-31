use poprako_orchestra::{Nucl, OperRun as _, OperStep as _, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::chapter_port::{
    ChapterImportComplex, ChapterPortPermComplex,
};
use crate::complex::unit::UnitComplex;
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::data::val::chapter_port::ImportChapterTranslationVal;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCounters;
use crate::model::shared::user::UserToken;
use crate::model::unit_port::UnitTranslationImport;
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
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::stage::spawn_starts;
use crate::value::chapter::Stage;
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

// Test suite for chapter import mapping and permission checks.
#[cfg(test)]
mod tests;

#[instrument(level = "info", err(Debug), skip(nucl, repo))]
/// Imports chapter translation content through the Unit edit pipeline.
pub async fn import<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: ImportChapterTranslationInstr,
    chapter_id: String,
) -> BaseRest<ImportChapterTranslationVal>
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

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?
    .ok_or_else(|| {
        //
        let err_message = trl("error-unit-edit-permission-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %token.user_id,
            operation = "import chapter translation",
            "expected error: unit edit permission required",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        }
    })?;

    let edit_perm = UnitEditPerm {
        can_translate: assignment_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR]),
        can_proofread: assignment_info
            .roles
            .has_any_role(&[RoleField::PROOFREADER]),
    };

    let label_plus = matches!(instr.format, TranslationFormat::LabelPlus);

    let imported_pages = match instr.format {
        //
        TranslationFormat::LabelPlus => {
            ChapterImportComplex::parse_label_plus(&instr.content)?
        }

        TranslationFormat::PopRaKo => {
            ChapterImportComplex::parse_poprako(&instr.content)?
        }
    };

    let stage_chapter_id = chapter_id.clone();

    let val = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let page_scopes = ListPageInfos {
                chapter_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            ChapterImportComplex::validate_page_count(
                imported_pages.len(),
                page_scopes.len(),
            )?;

            let mut imported_unit_count = 0;

            for (page_scope, imported_page) in
                page_scopes.iter().zip(imported_pages.iter())
            {
                let page_info = GetPageInfoExcluded { id: &page_scope.id }
                    .step_on(repo, context)
                    .await?;

                let orders = ListUnitOrders {
                    page_id: &page_info.id,
                }
                .step_on(repo, context)
                .await?;

                if imported_page.units.is_empty() {
                    continue;
                }

                let edits = build_page_edits(
                    &imported_page.units,
                    &token.user_id,
                    edit_perm,
                    label_plus,
                );

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

                let delta = page_counters(&page_info).calc_delta(counters);

                AdjustChapterUnitCounters {
                    id: &page_info.chapter_id,
                    delta,
                }
                .step_on(repo, context)
                .await?;

                imported_unit_count += imported_page.units.len() as i32;
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(ImportChapterTranslationVal {
                imported_page_count: page_scopes.len() as i32,
                imported_unit_count,
            })
        })
        .await?;

    let stages = import_stages(edit_perm);

    spawn_starts(((*repo).clone(),), stage_chapter_id, stages);

    accept(val)
}

// Builds page edits from imported units for scenario coverage.
fn build_page_edits(
    imported_units: &[UnitTranslationImport],
    user_id: &str,
    edit_perm: UnitEditPerm,
    label_plus: bool,
) -> Vec<UnitEdit> {
    // Build minimal page edits used by import scenario coverage.
    imported_units
        .iter()
        .map(|imported_unit| {
            ChapterImportComplex::build_unit_create(
                imported_unit,
                UnitComplex::gen_id(),
                user_id,
                edit_perm.can_translate,
                edit_perm.can_proofread,
                label_plus,
            )
        })
        .collect()
}

// Computes unit counters from page info for consistency checks.
fn page_counters(page_info: &PageInfo) -> UnitCounters {
    // Count page-level totals for consistency checks after import.
    UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    }
}

// Builds the repository execution stages required for import with permissions.
fn import_stages(edit_perm: UnitEditPerm) -> Vec<Stage> {
    //
    // Build the repository execution stages required for import with permissions.
    let mut stages = Vec::with_capacity(2);

    if edit_perm.can_translate {
        stages.push(Stage::Translate);
    }

    if edit_perm.can_proofread {
        stages.push(Stage::Proofread);
    }

    stages
}
