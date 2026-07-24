// FIXME: specific models and values are necessary.

use std::collections::HashMap;

use poprako_orchestra::run_proxy;
use tracing::instrument;

use crate::complex::chapter_port::{ChapterExportComplex, ChapterPortPermComplex};
use crate::data::chapter_port::ExportChapterTranslationPayload;
use crate::data::page_port::PageTranslationExportPayload;
use crate::data::unit_port::UnitTranslationExportPayload;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserToken;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListPageInfos;
use crate::part::repo::oper::unit::ListUnitInfos;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseResult, accept};
use crate::usecase::stage::spawn_starts;
use crate::value::chapter::Stage;

#[cfg(test)]
mod tests;

/// Exports one chapter as a JSON-safe translation payload.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn export<C, R>(
    repo: &R,
    token: UserToken,
    chapter_id: String,
) -> BaseResult<ExportChapterTranslationPayload>
where
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Clone
        + Send
        + Sync
        + 'static,
{
    ChapterPortPermComplex::ensure_user_can_export(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    let chapter_info = repo
        .run(&GetChapterInfo {
            id: &chapter_id,
            incls: &[],
        })
        .await?;

    let comic_info = repo
        .run(&GetComicInfo {
            id: &chapter_info.comic_id,
            incls: &[],
        })
        .await?;

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &chapter_info.id,
        })
        .await?;

    let mut page_vals = Vec::with_capacity(page_infos.len());

    for page_info in page_infos {
        //

        let unit_infos = repo
            .run(&ListUnitInfos {
                page_id: &page_info.id,
            })
            .await?;

        let unit_vals = unit_infos
            .into_iter()
            .map(|unit_info| make_unit_export(&page_info, unit_info))
            .collect();

        page_vals.push(PageTranslationExportPayload {
            page_id: page_info.id,
            page_index: page_info.index,
            units: unit_vals,
        });
    }

    let payload = ExportChapterTranslationPayload {
        chapter_id: chapter_info.id,
        chapter_index: chapter_info.index,
        chapter_subtitle: non_empty(chapter_info.subtitle),
        comic_id: chapter_info.comic_id,
        comic_title: comic_info.title,
        pages: page_vals,
    };

    spawn_starts(
        (*repo).clone(),
        payload.chapter_id.clone(),
        vec![Stage::TypesetRedraw],
    );

    accept(payload)
}

/// Exports one chapter as LabelPlus text.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn export_label_plus<C, R>(
    repo: &R,
    token: UserToken,
    chapter_id: String,
) -> BaseResult<String>
where
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Clone
        + Send
        + Sync
        + 'static,
{
    ChapterPortPermComplex::ensure_user_can_export(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    repo.run(&GetChapterInfo {
        id: &chapter_id,
        incls: &[],
    })
    .await?;

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &chapter_id,
        })
        .await?;

    let mut units_by_page_id = HashMap::new();

    for page_info in &page_infos {
        //

        let unit_infos = repo
            .run(&ListUnitInfos {
                page_id: &page_info.id,
            })
            .await?;

        units_by_page_id.insert(page_info.id.clone(), unit_infos);
    }

    let content =
        ChapterExportComplex::make_label_plus(&page_infos, &units_by_page_id);

    spawn_starts((*repo).clone(), chapter_id, vec![Stage::TypesetRedraw]);

    accept(content)
}

/// Builds a [`UnitTranslationExportVal`] from page and unit info.
fn make_unit_export(
    page_info: &PageInfo,
    unit_info: UnitInfo,
) -> UnitTranslationExportPayload {
    UnitTranslationExportPayload {
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
