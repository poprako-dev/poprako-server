// Test suite for chapter import mapping and perm checks.
#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::chapter_port::import::ChapterImportComplex;
use crate::complex::chapter_port::perm::ChapterPortPermComplex;
use crate::complex::unit::UnitComplex;
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::data::val::chapter_port::ImportChapterTranslationVal;
use crate::model::page_port::PageTranslationImport;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitOrder};
use crate::model::shared::user::UserToken;
use crate::model::unit_port::UnitTranslationImport;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    GetPageInfoExcluded, ListPageInfos, SetPageUnitCounters,
};
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter::stage::Stage;
use crate::value::chapter_port::TranslationFormat;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleField;
use crate::value::unit::UnitEditPerm;

/// Imports chapter translation content through the Unit edit pipeline.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn import<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: ImportChapterTranslationInstr,
    chapter_id: String,
) -> BaseRest<ImportChapterTranslationVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let (edit_perm, format, label_plus, imported_pages, stages) =
        prepare_import(repo, &token, &instr, &chapter_id).await?;

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

            let mut final_page_counters = Vec::with_capacity(page_scopes.len());

            for page_scope in &page_scopes {
                //
                let imported_page = imported_pages
                    .iter()
                    .find(|page| page.page_index == page_scope.index)
                    .ok_or_else(|| BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-invalid-chapter-import-content"),
                    })?;

                let final_counters = replace_page_units(
                    repo,
                    context,
                    &page_scope.id,
                    imported_page,
                    &token.user_id,
                    edit_perm,
                    label_plus,
                )
                .await?;

                final_page_counters.push(final_counters);

                imported_unit_count += imported_page.units.len();
            }

            let chapter_counters = final_page_counters.iter().copied().fold(
                UnitCountMetrics::default(),
                |mut counters, page_counters| {
                    //
                    counters.total += page_counters.total;

                    counters.translated += page_counters.translated;

                    counters.proofread += page_counters.proofread;

                    counters
                },
            );

            SetChapterPageCounters {
                id: &chapter_info.id,
                page_count: page_scopes.len(),
                total_unit_count: chapter_counters.total,
                translated_unit_count: chapter_counters.translated,
                proofread_unit_count: chapter_counters.proofread,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let import_val = ImportChapterTranslationVal {
                imported_page_count: page_scopes.len(),
                imported_unit_count,
            };

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(token.user_id.clone()),
                ChapterWorkflowRecordPayload::TranslationImported {
                    format,
                    imported_page_count: import_val.imported_page_count,
                    imported_unit_count: import_val.imported_unit_count,
                },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&workflow_record_entry),
            }
            .step_on(repo, context)
            .await?;

            start_pending_stages(
                repo,
                context,
                &chapter_info.id,
                Some(token.user_id.clone()),
                ChapterWorkflowRecordOrigin::TranslationImport,
                &stages,
            )
            .await?;

            accept(import_val)
        })
        .await?;

    accept(val)
}

// Validates permissions and parses the submitted translation content.
async fn prepare_import<C, R>(
    repo: &R,
    token: &UserToken,
    instr: &ImportChapterTranslationInstr,
    chapter_id: &str,
) -> BaseRest<(
    UnitEditPerm,
    TranslationFormat,
    bool,
    Vec<PageTranslationImport>,
    Vec<Stage>,
)>
where
    C: Context,
    R: AssignmentRepo<C> + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?
    .ok_or_else(|| {
        //
        let err_message = trl("error-chapter-port-import-perm-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id,
            user_id = %token.user_id,
            operation = "import chapter translation",
            "expected error: chapter import assignment required",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        }
    })?;

    ChapterPortPermComplex::ensure_user_can_import(&assignment_info)?;

    let edit_perm = UnitEditPerm {
        can_translate: assignment_info
            .roles
            .has_any_role(&[RoleField::TRANSLATOR]),
        can_proofread: assignment_info
            .roles
            .has_any_role(&[RoleField::PROOFREADER]),
    };

    let format = TranslationFormat::from(instr.format);

    let label_plus = matches!(format, TranslationFormat::LabelPlus);

    let imported_pages = match format {
        //
        TranslationFormat::LabelPlus => {
            ChapterImportComplex::parse_label_plus(&instr.content)?
        }

        TranslationFormat::PopRaKo => {
            ChapterImportComplex::parse_poprako(&instr.content)?
        }
    };

    let stages = import_stages(edit_perm);

    accept((edit_perm, format, label_plus, imported_pages, stages))
}

// Replaces all visible units on one page with imported units.
async fn replace_page_units<C, R>(
    repo: &R,
    context: &mut C,
    page_id: &str,
    imported_page: &PageTranslationImport,
    user_id: &str,
    edit_perm: UnitEditPerm,
    label_plus: bool,
) -> BaseRest<UnitCountMetrics>
where
    C: Context,
    R: PageRepo<C> + UnitRepo<C> + Sync,
{
    let page_info = GetPageInfoExcluded { id: page_id }
        .step_on(repo, context)
        .await?;

    let orders = ListUnitOrders {
        page_id: &page_info.id,
    }
    .step_on(repo, context)
    .await?;

    let visible_unit_ids = orders
        .iter()
        .filter(|order| !order.is_hidden)
        .map(|order| order.id.clone())
        .collect::<Vec<_>>();

    if !visible_unit_ids.is_empty() {
        //
        let delete_edits = visible_unit_ids
            .iter()
            .map(|id| UnitEdit::Delete { id: id.clone() })
            .collect::<Vec<_>>();

        let base_ids = orders
            .iter()
            .map(|order| order.id.as_str())
            .collect::<Vec<_>>();

        let delete_edits =
            UnitComplex::normalize_edits(&base_ids, delete_edits)?;

        ApplyUnitEdits {
            page_id: &page_info.id,
            orders: &orders,
            edits: &delete_edits,
        }
        .step_on(repo, context)
        .await?;
    }

    let orders = ListUnitOrders {
        page_id: &page_info.id,
    }
    .step_on(repo, context)
    .await?;

    let final_counters = apply_imported_units(
        repo,
        context,
        (&page_info.id, &orders, imported_page),
        (user_id, edit_perm, label_plus),
    )
    .await?;

    SetPageUnitCounters {
        id: &page_info.id,
        counters: final_counters,
    }
    .step_on(repo, context)
    .await?;

    accept(final_counters)
}

// Selects workflow stages that the importing assignment can edit.
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

// Applies imported units against the current page order.
async fn apply_imported_units<C, R>(
    repo: &R,
    context: &mut C,
    (page_id, orders, imported_page): (
        &str,
        &[UnitOrder],
        &PageTranslationImport,
    ),
    (user_id, edit_perm, label_plus): (&str, UnitEditPerm, bool),
) -> BaseRest<UnitCountMetrics>
where
    C: Context,
    R: UnitRepo<C> + Sync,
{
    let Some(_) = imported_page.units.first() else {
        return accept(UnitCountMetrics::default());
    };

    let edits =
        build_page_edits(&imported_page.units, user_id, edit_perm, label_plus);

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let edits = UnitComplex::normalize_edits(&base_ids, edits)?;

    let final_counters = ApplyUnitEdits {
        page_id,
        orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    accept(final_counters)
}

// Builds create edits for all imported units on one page.
fn build_page_edits(
    imported_units: &[UnitTranslationImport],
    user_id: &str,
    edit_perm: UnitEditPerm,
    label_plus: bool,
) -> Vec<UnitEdit> {
    //
    imported_units
        .iter()
        .map(|imported_unit| {
            //
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
