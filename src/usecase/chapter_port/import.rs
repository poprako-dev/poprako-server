#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::chapter_port::{ChapterImportComplex, ChapterPortPermComplex};
use crate::complex::unit::UnitComplex;
use crate::data::chapter_port::{ChapterTranslationImportData, ChapterTranslationImportVal};
use crate::model::assignment::AssignmentInfo;
use crate::model::page::PageInfo;
use crate::model::unit::{UnitCounterDelta, UnitCounters, UnitInfo, UnitOper};
use crate::model::unit_port::UnitTranslationImport;
use crate::model::user::UserToken;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::unit::UnitStep;
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::result::{ExpectedVariant, RegularError, RegularResult, accept};
use crate::util::DeriveTransactional;
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::RoleField;

/// Imports chapter translation text into existing pages.
pub async fn import<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: ChapterTranslationImportData,
    chapter_id: String,
) -> RegularResult<ChapterTranslationImportVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentRepo<C> + ChapterRepo<C> + ComicRepo<C> + PageRepo<C> + UnitRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + PageRepoTransactional<C>
        + UnitRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPortPermComplex::can_user_import(&mut repo.as_proxy(), &token.user_id, &chapter_id)
        .await?;

    let assignment_info = repo
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            &chapter_id,
            &token.user_id,
        ))
        .await?
        .ok_or_else(unit_edit_permission_error)?;

    let label_plus = matches!(data.format, TranslationFormat::LabelPlus);

    let imported_pages = match data.format {
        TranslationFormat::LabelPlus => ChapterImportComplex::parse_label_plus(&data.content)?,
        TranslationFormat::PopRaKo => ChapterImportComplex::parse_poprako(&data.content)?,
    };

    let imported = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let chapter_info = repo
                .advance(context, &ChapterStep::get_info_by_id(&chapter_id, &[]))
                .await?;

            let page_infos = repo
                .advance(
                    context,
                    &PageStep::list_all_infos_by_chapter_id(&chapter_id),
                )
                .await?;

            ChapterImportComplex::validate_page_count(imported_pages.len(), page_infos.len())?;

            let mut imported_unit_count = 0;

            for (page_info, imported_page) in page_infos.iter().zip(imported_pages.iter()) {
                let old_counters = page_counters(page_info);

                let existing_unit_infos = repo
                    .advance(context, &UnitStep::list_infos_by_page_id(&page_info.id))
                    .await?;

                let existing_by_id = existing_unit_infos
                    .iter()
                    .map(|unit_info| (unit_info.id.as_str(), unit_info))
                    .collect::<HashMap<_, _>>();

                let existing_by_index = existing_unit_infos
                    .iter()
                    .map(|unit_info| (unit_info.index, unit_info))
                    .collect::<HashMap<_, _>>();

                let mut candidate_order = Vec::with_capacity(imported_page.units.len());

                for imported_unit in &imported_page.units {
                    let unit_id = resolve_unit_id(imported_unit, &existing_by_index);

                    let existing_unit = existing_by_id
                        .get(unit_id.as_str())
                        .copied()
                        .or_else(|| existing_by_index.get(&imported_unit.index).copied());

                    let unit_payload = ChapterImportComplex::build_unit_payload(
                        imported_unit,
                        existing_unit,
                        &token.user_id,
                        has_proofreader_role(&assignment_info),
                        label_plus,
                    );

                    let unit_oper = UnitOper::Save {
                        id: unit_id.clone(),
                        payload: unit_payload,
                    };

                    repo.advance(context, &UnitStep::save_info(&page_info.id, &unit_oper))
                        .await?;

                    candidate_order.push(unit_id);
                }

                let current_indexes = repo
                    .advance(context, &UnitStep::list_indexes_by_page_id(&page_info.id))
                    .await?;

                let index_updates =
                    UnitComplex::build_index_updates(&candidate_order, &[], current_indexes);

                if !index_updates.is_empty() {
                    repo.advance(
                        context,
                        &UnitStep::update_indexes_by_page_id(&page_info.id, &index_updates),
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

                let delta = counter_delta(old_counters, counters);

                repo.advance(
                    context,
                    &ChapterStep::adjust_unit_counters(&page_info.chapter_id, delta),
                )
                .await?;

                imported_unit_count += imported_page.units.len() as i32;
            }

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            accept(ChapterTranslationImportVal {
                imported_page_count: page_infos.len() as i32,
                imported_unit_count,
            })
        })
        .await
        .map_err(map_drive_err)?;

    accept(imported)
}

fn page_counters(page_info: &PageInfo) -> UnitCounters {
    UnitCounters {
        total_unit_count: page_info.total_unit_count,
        translated_unit_count: page_info.translated_unit_count,
        proofread_unit_count: page_info.proofread_unit_count,
    }
}

fn resolve_unit_id(
    imported_unit: &UnitTranslationImport,
    existing_by_index: &HashMap<i32, &UnitInfo>,
) -> String {
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

fn has_proofreader_role(assignment_info: &AssignmentInfo) -> bool {
    assignment_info
        .roles
        .has_any_role(&[RoleField::PROOFREADER])
}

fn counter_delta(old_counters: UnitCounters, new_counters: UnitCounters) -> UnitCounterDelta {
    UnitCounterDelta {
        total_unit_count: new_counters.total_unit_count - old_counters.total_unit_count,
        translated_unit_count: new_counters.translated_unit_count
            - old_counters.translated_unit_count,
        proofread_unit_count: new_counters.proofread_unit_count - old_counters.proofread_unit_count,
    }
}

fn unit_edit_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::PermDeny,
        message: trl("error-unit-edit-permission-required"),
    }
}
