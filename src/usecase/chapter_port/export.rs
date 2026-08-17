// Test coverage for chapter export payload shape and ordering.
#[cfg(test)]
mod tests;

// FIXME: specific models and values are necessary.

use std::collections::HashMap;

use poprako_orchestra::{
    AtLeast, Nucl, OperRun as _, OperStep as _, run_proxy,
};
use tracing::instrument;

use crate::complex::chapter_port::{
    ChapterExportComplex, ChapterPortPermComplex,
};
use crate::data::val::chapter_port::ExportChapterTranslationVal;
use crate::data::view::page_port::PageTranslationExportView;
use crate::data::view::unit_port::UnitTranslationExportView;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListPageInfos;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::unit::ListUnitInfos;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::stage::start_pending_stages;
use crate::value::chapter::Stage;
use crate::value::chapter_port::TranslationFormat;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Exports one chapter as a JSON-safe translation payload.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn export<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<ExportChapterTranslationVal>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    ChapterPortPermComplex::ensure_user_can_export(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    let chapter_info = GetChapterInfo {
        id: &chapter_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let comic_info = GetComicInfo {
        id: &chapter_info.comic_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let page_infos = ListPageInfos {
        chapter_id: &chapter_info.id,
    }
    .run_on(repo)
    .await?;

    let mut page_views = Vec::with_capacity(page_infos.len());

    for page_info in page_infos {
        //
        // Load visible units for each page and map them into exported unit views.

        let unit_infos = ListUnitInfos {
            page_id: &page_info.id,
        }
        .run_on(repo)
        .await?;

        let unit_views = unit_infos
            .into_iter()
            .filter(|unit_info| unit_info.hidden_at.is_none())
            .enumerate()
            .map(|(index, unit_info)| {
                make_unit_export(&page_info, index, unit_info)
            })
            .collect();

        page_views.push(PageTranslationExportView {
            page_id: page_info.id,
            page_index: page_info.index,
            units: unit_views,
        });
    }

    let val = ExportChapterTranslationVal {
        chapter_id: chapter_info.id,
        chapter_index: chapter_info.index,
        chapter_subtitle: non_empty(chapter_info.subtitle),
        comic_id: chapter_info.comic_id,
        comic_title: comic_info.title,
        pages: page_views,
    };

    persist_export_record(
        (nucl, repo),
        val.chapter_id.clone(),
        token.user_id,
        TranslationFormat::PopRaKo,
    )
    .await?;

    accept(val)
}

/// Exports one chapter as LabelPlus text.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn export_label_plus<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<String>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
{
    ChapterPortPermComplex::ensure_user_can_export(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    GetChapterInfo {
        id: &chapter_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let page_infos = ListPageInfos {
        chapter_id: &chapter_id,
    }
    .run_on(repo)
    .await?;

    let mut units_by_page_id = HashMap::new();

    for page_info in &page_infos {
        //
        // Collect visible units grouped by page before LabelPlus serialization.

        let unit_infos = ListUnitInfos {
            page_id: &page_info.id,
        }
        .run_on(repo)
        .await?;

        let unit_infos = unit_infos
            .into_iter()
            .filter(|unit_info| unit_info.hidden_at.is_none())
            .collect();

        units_by_page_id.insert(page_info.id.clone(), unit_infos);
    }

    let content =
        ChapterExportComplex::make_label_plus(&page_infos, &units_by_page_id);

    persist_export_record(
        (nucl, repo),
        chapter_id,
        token.user_id,
        TranslationFormat::LabelPlus,
    )
    .await?;

    accept(content)
}

// Builds a [`UnitTranslationExportView`] from page and unit info.
fn make_unit_export(
    page_info: &PageInfo,
    index: usize,
    unit_info: UnitInfo,
) -> UnitTranslationExportView {
    //
    // Convert one unit into export view fields used by downstream translators.
    UnitTranslationExportView {
        unit_id: unit_info.id,
        unit_index: index as i32,
        page_id: page_info.id.clone(),
        page_index: page_info.index,
        x_coord: unit_info.coord.x_coord,
        y_coord: unit_info.coord.y_coord,
        is_bubble: unit_info.is_bubble,
        translated_text: unit_info.translated_text,
        translator_id: unit_info.last_translator_id,
        is_proofread: unit_info.is_proofread,
        proofread_text: unit_info.proofread_text,
        proofreader_id: unit_info.last_proofreader_id,
    }
}

// Returns [`Some`] with the text if non-empty, [`None`] otherwise.
fn non_empty(text: String) -> Option<String> {
    //
    // Trim and normalize optional text fields before sending user-facing translation payloads.
    if text.trim().is_empty() {
        return None;
    }

    Some(text)
}

// Persists a completed export and starts typesetting/redraw in one transaction.
async fn persist_export_record<N, C, R>(
    (nucl, repo): (&N, &R),
    chapter_id: String,
    actor_user_id: String,
    format: TranslationFormat,
) -> BaseRest<()>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C> + ChapterWorkflowRecordRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                chapter_info.id.clone(),
                Some(actor_user_id.clone()),
                ChapterWorkflowRecordPayload::TranslationExported { format },
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
                Some(actor_user_id),
                ChapterWorkflowRecordOrigin::TranslationExport,
                &[Stage::TypesetRedraw],
            )
            .await?;

            accept(())
        })
        .await?;

    accept(())
}
