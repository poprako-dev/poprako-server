// Test suite for chapter import mapping and perm checks.
#[cfg(test)]
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter_port::{
    import_translation as chapter_translation_import_complex,
    perm as chapter_port_perm_complex,
};
use crate::complex::unit::perm as unit_perm_complex;
use crate::complex::{chapter as chapter_complex, unit as unit_complex};
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;
use crate::data::val::chapter_port::ImportChapterTranslationVal;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::read::proj::unit::{UnitCountMetrics, UnitOrder};
use crate::model::shared::user::UserToken;
use crate::model::write::translation_import::{
    PageSourceTranslationImport, UnitTranslationImport,
};
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageSourceCountMetrics,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    ListPageSourceInfosExcluded, SetPageSourceUnitCountMetrics,
};
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::chapter_workflow_record::ChapterWorkflowRecordEntryShape;
use crate::usecase::stage as stage_usecase;
use crate::value::chapter::stage::Stage;
use crate::value::chapter_port::{
    ChapterTranslationImportMode, TranslationFormat,
};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::unit::UnitEditPerm;

/// Imports chapter translation content through the Unit edit pipeline.
#[instrument(
    level = "info",
    skip(nucl, repo, token, instr),
    fields(
        actor_user_id = %token.user_id,
        format = ?instr.format,
        mode = ?instr.mode,
        content_byte_len = instr.content.len(),
    ),
)]
pub async fn import_translation<N, C, R>(
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
        + PageSourceRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    let (edit_perm, format, mode, imported_page_sources, stages) =
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

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            let page_source_scopes = ListPageSourceInfosExcluded {
                chapter_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            chapter_translation_import_complex::validate_page_source_count(
                imported_page_sources.len(),
                page_source_scopes.len(),
            )?;

            let import_settings = ImportSettings {
                user_id: &token.user_id,
                edit_perm,
                mode,
            };

            let (
                final_page_source_count_metrics,
                imported_page_source_count,
                imported_unit_count,
            ) = import_page_sources(
                repo,
                context,
                &page_source_scopes,
                &imported_page_sources,
                &import_settings,
            )
            .await?;

            let chapter_count_metrics =
                final_page_source_count_metrics.iter().copied().fold(
                    UnitCountMetrics::default(),
                    |mut count_metrics, page_source_count_metrics| {
                        //
                        count_metrics.total += page_source_count_metrics.total;

                        count_metrics.translated +=
                            page_source_count_metrics.translated;

                        count_metrics.proofread +=
                            page_source_count_metrics.proofread;

                        count_metrics
                    },
                );

            SetChapterPageSourceCountMetrics {
                id: &chapter_info.id,
                page_source_count: page_source_scopes.len(),
                total_unit_count: chapter_count_metrics.total,
                translated_unit_count: chapter_count_metrics.translated,
                proofread_unit_count: chapter_count_metrics.proofread,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let import_val = ImportChapterTranslationVal {
                imported_page_source_count,
                imported_unit_count,
            };

            let entry = ChapterWorkflowRecordEntryShape::new(
                chapter_info.id.as_str(),
                Some(token.user_id.as_str()),
                ChapterWorkflowRecordPayload::TranslationImported {
                    format,
                    imported_page_source_count: import_val
                        .imported_page_source_count,
                    imported_unit_count: import_val.imported_unit_count,
                },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&entry),
            }
            .step_on(repo, context)
            .await?;

            stage_usecase::start_pending_stages(
                repo,
                context,
                stage_usecase::PendingStageStart::new(
                    &chapter_info.id,
                    Some(&token.user_id),
                    ChapterWorkflowRecordOrigin::TranslationImport,
                ),
                &stages,
            )
            .await?;

            accept!(import_val)
        })
        .await?;

    accept!(val)
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
    ChapterTranslationImportMode,
    Vec<PageSourceTranslationImport>,
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
        expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-chapter-port-import-perm-required");
            chapter_id,
            user_id = %token.user_id,
            operation = "import chapter translation",
            "chapter import assignment required",
        }
    })?;

    chapter_port_perm_complex::ensure_user_can_import_translation(
        &assignment_info,
    )?;

    let edit_perm = unit_perm_complex::edit_perm(Some(&assignment_info));

    let format = TranslationFormat::from(instr.format);

    let mode = ChapterTranslationImportMode::from(instr.mode);

    let imported_page_sources = match format {
        //
        TranslationFormat::LabelPlus => {
            //
            chapter_translation_import_complex::parse_label_plus(
                &instr.content,
            )?
        }

        TranslationFormat::PopRaKo => {
            chapter_translation_import_complex::parse_poprako(&instr.content)?
        }
    };

    let mut stages = import_stages(edit_perm);

    if !stages.contains(&Stage::Translate)
        && imported_page_sources
            .iter()
            .flat_map(|page| &page.units)
            .any(|unit| {
                //
                unit.typography_note
                    .as_ref()
                    .is_some_and(|note| !note.trim().is_empty())
            })
    {
        stages.insert(0, Stage::Translate);
    }

    accept!((edit_perm, format, mode, imported_page_sources, stages))
}

// Carries the actor and strategy through each page import.
struct ImportSettings<'a> {
    // User responsible for imported Unit edits.
    user_id: &'a str,
    // Fields the user may import.
    edit_perm: UnitEditPerm,
    // Strategy for existing Unit content.
    mode: ChapterTranslationImportMode,
}

// Applies imported pages and collects their counters and import totals.
async fn import_page_sources<C, R>(
    repo: &R,
    context: &mut C,
    page_source_scopes: &[PageSourceInfo],
    imported_page_sources: &[PageSourceTranslationImport],
    import_settings: &ImportSettings<'_>,
) -> BaseRest<(Vec<UnitCountMetrics>, usize, usize)>
where
    C: Context,
    R: PageSourceRepo<C> + UnitRepo<C> + Sync,
{
    let mut imported_page_source_count = 0;

    let mut imported_unit_count = 0;

    let mut final_page_source_count_metrics =
        Vec::with_capacity(page_source_scopes.len());

    for page_source_scope in page_source_scopes {
        //
        let imported_page = imported_page_sources
            .iter()
            .find(|page| page.page_source_index == page_source_scope.index)
            .ok_or_else(|| {
                //
                expected_error! {
                    ExpectedVariant::Args,
                    message: trl("error-invalid-chapter-import-content");
                    "imported chapter page is missing",
                }
            })?;

        let page_source_import_changes = apply_page_source_import(
            repo,
            context,
            page_source_scope,
            imported_page,
            import_settings,
        )
        .await?;

        final_page_source_count_metrics
            .push(page_source_import_changes.count_metrics);

        imported_page_source_count +=
            page_source_import_changes.imported_page_source_count;

        imported_unit_count += page_source_import_changes.imported_unit_count;
    }

    accept!((
        final_page_source_count_metrics,
        imported_page_source_count,
        imported_unit_count,
    ))
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

// Captures the page-level results of one import application decision.
struct PageSourceImportChanges {
    // Final visible Unit counters for the page.
    count_metrics: UnitCountMetrics,
    // Whether this page's visible content changed through the import.
    imported_page_source_count: usize,
    // Number of source Units created through the import.
    imported_unit_count: usize,
}

// Applies imported Units to one page according to the selected strategy.
async fn apply_page_source_import<C, R>(
    repo: &R,
    context: &mut C,
    page_source_scope: &PageSourceInfo,
    imported_page: &PageSourceTranslationImport,
    import_settings: &ImportSettings<'_>,
) -> BaseRest<PageSourceImportChanges>
where
    C: Context,
    R: PageSourceRepo<C> + UnitRepo<C> + Sync,
{
    let orders = ListUnitOrders {
        page_source_id: &page_source_scope.id,
    }
    .step_on(repo, context)
    .await?;

    let visible_unit_ids = orders
        .iter()
        .filter(|order| !order.is_hidden)
        .map(|order| order.id.clone())
        .collect::<Vec<_>>();

    let current_count_metrics =
        page_source_unit_count_metrics(page_source_scope);

    if (visible_unit_ids.is_empty(), imported_page.units.is_empty())
        == (true, true)
    {
        return accept!(PageSourceImportChanges {
            count_metrics: current_count_metrics,
            imported_page_source_count: 0,
            imported_unit_count: 0,
        });
    }

    match import_settings.mode {
        //
        ChapterTranslationImportMode::Keep if !visible_unit_ids.is_empty() => {
            //
            return accept!(PageSourceImportChanges {
                count_metrics: current_count_metrics,
                imported_page_source_count: 0,
                imported_unit_count: 0,
            });
        }

        _ => {}
    }

    let orders = match import_settings.mode {
        //
        ChapterTranslationImportMode::Overwrite
            if !visible_unit_ids.is_empty() =>
        {
            hide_visible_units(
                repo,
                context,
                &page_source_scope.id,
                &orders,
                &visible_unit_ids,
            )
            .await?;

            orders
                .into_iter()
                .map(|mut order| {
                    //
                    order.is_hidden = true;

                    order
                })
                .collect()
        }

        ChapterTranslationImportMode::Keep
        | ChapterTranslationImportMode::Overwrite => orders,
    };

    let final_count_metrics = apply_imported_units(
        repo,
        context,
        (&page_source_scope.id, &orders, imported_page),
        (import_settings.user_id, import_settings.edit_perm),
    )
    .await?;

    SetPageSourceUnitCountMetrics {
        id: &page_source_scope.id,
        count_metrics: final_count_metrics,
    }
    .step_on(repo, context)
    .await?;

    accept!(PageSourceImportChanges {
        count_metrics: final_count_metrics,
        imported_page_source_count: 1,
        imported_unit_count: imported_page.units.len(),
    })
}

// Returns the stored counters for a page whose visible Units are unchanged.
const fn page_source_unit_count_metrics(
    page_source_info: &PageSourceInfo,
) -> UnitCountMetrics {
    UnitCountMetrics {
        total: page_source_info.total_unit_count,
        translated: page_source_info.translated_unit_count,
        proofread: page_source_info.proofread_unit_count,
    }
}

// Hides every currently visible Unit before an overwrite import.
async fn hide_visible_units<C, R>(
    repo: &R,
    context: &mut C,
    page_source_id: &str,
    orders: &[UnitOrder],
    visible_unit_ids: &[String],
) -> BaseRest<()>
where
    C: Context,
    R: UnitRepo<C> + Sync,
{
    let delete_edits = visible_unit_ids
        .iter()
        .map(|id| UnitEdit::Delete { id: id.clone() })
        .collect::<Vec<_>>();

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let delete_edits = unit_complex::normalize_edits(&base_ids, delete_edits)?;

    ApplyUnitEdits {
        page_source_id,
        orders,
        edits: &delete_edits,
    }
    .step_on(repo, context)
    .await?;

    accept!(())
}

// Applies imported units against the current page order.
async fn apply_imported_units<C, R>(
    repo: &R,
    context: &mut C,
    (page_source_id, orders, imported_page): (
        &str,
        &[UnitOrder],
        &PageSourceTranslationImport,
    ),
    (user_id, edit_perm): (&str, UnitEditPerm),
) -> BaseRest<UnitCountMetrics>
where
    C: Context,
    R: UnitRepo<C> + Sync,
{
    let Some(_) = imported_page.units.first() else {
        return accept!(UnitCountMetrics::default());
    };

    let edits =
        build_page_source_edits(&imported_page.units, user_id, edit_perm);

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let edits = unit_complex::normalize_edits(&base_ids, edits)?;

    let final_count_metrics = ApplyUnitEdits {
        page_source_id,
        orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    accept!(final_count_metrics)
}

// Builds create edits for all imported units on one page.
fn build_page_source_edits(
    imported_units: &[UnitTranslationImport],
    user_id: &str,
    edit_perm: UnitEditPerm,
) -> Vec<UnitEdit> {
    //
    imported_units
        .iter()
        .map(|imported_unit| {
            //
            chapter_translation_import_complex::build_unit_create(
                imported_unit,
                unit_complex::gen_id(),
                user_id,
                edit_perm.can_translate,
                edit_perm.can_proofread,
            )
        })
        .collect()
}
