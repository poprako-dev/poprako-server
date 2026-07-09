#[cfg(test)]
mod tests;

// FIXME: specific models and values are necessary.

use std::collections::HashMap;

use crate::complex::chapter_port::{
    ChapterExportComplex, ChapterPortPermComplex,
};
use crate::data::chapter_port::ChapterTranslationExportVal;
use crate::data::page_port::PageTranslationExportVal;
use crate::data::unit_port::UnitTranslationExportVal;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
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
use crate::result::RegularResult;
use crate::util::DeriveTransactional;

/// Exports one chapter as a JSON-safe translation payload.
pub async fn export<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    chapter_id: String,
) -> RegularResult<ChapterTranslationExportVal>
where
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + PageRepoTransactional<C>
        + UnitRepoTransactional<C>,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPortPermComplex::can_user_export(
        &mut repo.as_proxy(),
        &token.user_id,
        &chapter_id,
    )
    .await?;

    let chapter_info = repo
        .execute(&ChapterStep::get_info_by_id(&chapter_id, &[]))
        .await?;

    let comic_info = repo
        .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id, &[]))
        .await?;

    let page_infos = repo
        .execute(&PageStep::list_all_infos_by_chapter_id(&chapter_info.id))
        .await?;

    let mut page_vals = Vec::with_capacity(page_infos.len());

    for page_info in page_infos {
        //
        let unit_infos = repo
            .execute(&UnitStep::list_all_infos_by_page_id(&page_info.id))
            .await?;

        let image_url = match (page_info.image_uploaded, &page_info.image_key) {
            (true, Some(image_key)) => {
                image_pool.get_signed(image_key).await.ok()
            }
            _ => None,
        };

        let unit_vals = unit_infos
            .into_iter()
            .map(|unit_info| make_unit_export(&page_info, unit_info))
            .collect();

        page_vals.push(PageTranslationExportVal {
            page_id: page_info.id,
            page_index: page_info.index,
            image_url: image_url.map(Into::into),
            units: unit_vals,
        });
    }

    Ok(ChapterTranslationExportVal {
        chapter_id: chapter_info.id,
        chapter_index: chapter_info.index,
        chapter_subtitle: non_empty(chapter_info.subtitle),
        comic_id: chapter_info.comic_id,
        comic_title: comic_info.title,
        pages: page_vals,
    })
}

/// Exports one chapter as LabelPlus text.
pub async fn export_label_plus<C, R>(
    repo: &R,
    token: UserToken,
    chapter_id: String,
) -> RegularResult<String>
where
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + PageRepoTransactional<C>
        + UnitRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPortPermComplex::can_user_export(
        &mut repo.as_proxy(),
        &token.user_id,
        &chapter_id,
    )
    .await?;

    repo.execute(&ChapterStep::get_info_by_id(&chapter_id, &[]))
        .await?;

    let page_infos = repo
        .execute(&PageStep::list_all_infos_by_chapter_id(&chapter_id))
        .await?;

    let mut units_by_page_id = HashMap::new();

    for page_info in &page_infos {
        //
        let unit_infos = repo
            .execute(&UnitStep::list_all_infos_by_page_id(&page_info.id))
            .await?;

        units_by_page_id.insert(page_info.id.clone(), unit_infos);
    }

    Ok(ChapterExportComplex::make_label_plus(
        &page_infos,
        &units_by_page_id,
    ))
}

/// Builds a [`UnitTranslationExportVal`] from page and unit info.
fn make_unit_export(
    page_info: &PageInfo,
    unit_info: UnitInfo,
) -> UnitTranslationExportVal {
    UnitTranslationExportVal {
        unit_id: unit_info.id,
        unit_index: unit_info.index,
        page_id: page_info.id.clone(),
        page_index: page_info.index,
        x_coord: unit_info.x_coord,
        y_coord: unit_info.y_coord,
        is_bubble: unit_info.is_bubble,
        translated_text: unit_info.translated_text,
        translator_id: unit_info.last_translator_id,
        is_proofread: unit_info.is_proofread,
        proofread_text: unit_info.proofread_text,
        proofreader_id: unit_info.last_proofreader_id,
    }
}

/// Returns [`Some`] with the text if non-empty, [`None`] otherwise.
fn non_empty(text: String) -> Option<String> {
    //
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}
