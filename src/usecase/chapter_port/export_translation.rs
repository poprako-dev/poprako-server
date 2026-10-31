// Test coverage for chapter export payload shape and ordering.
#[cfg(test)]
mod tests;

// Export data uses the shared translation-document models and validated format spec.

use std::collections::HashMap;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::oper::ListObjMetas;

use crate::complex::chapter_port::export_translation as chapter_translation_export_complex;
use crate::data::val::chapter_port::{
    ChapterPageSourceRawIdentVal, ExportChapterTranslationsVal,
};
use crate::data::view::chapter_port::ChapterTranslationPortView;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceRawIdentInfo,
};
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::translation_document::{
    ChapterTranslationDocument, PageSourceTranslationDocument,
    UnitTranslationDocument,
};
use crate::model::shared::user::UserToken;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageSourceImage;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::page_source::{
    ListPageSourceInfos, ListPageSourceRawIdentInfos,
};
use crate::part::repo::oper::unit::ListUnitInfosByPageSourceIds;
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseError, BaseRest};
use crate::shape::write::chapter_workflow_record::ChapterWorkflowRecordEntryShape;
use crate::usecase::chapter_port::perm as chapter_port_perm_usecase;
use crate::usecase::stage as stage_usecase;
use crate::value::business_id::PageSourceId;
use crate::value::chapter::stage::Stage;
use crate::value::chapter_port::ExportFormatSpec;
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

// Borrows loaded chapter, page, and object data during synchronous assembly.
struct ChapterExportSnapshot<'a> {
    // Chapter identity and the owned comic title moved into the native document.
    chapter_info: &'a ChapterInfo,
    // Comic title moved into the native export document.
    comic_title: String,

    // Ordered content and source filenames from the same read snapshot.
    page_source_infos: &'a [PageSourceInfo],
    // Translation units grouped into pages during assembly.
    unit_infos: Vec<UnitInfo>,
    // Original filenames read alongside the chapter pages.
    raw_ident_infos: &'a [PageSourceRawIdentInfo],

    // Metadata used to render page image extensions without another lookup.
    obj_metas: &'a HashMap<PageSourceId, ObjMeta<PageSourceImage>>,
}

/// Exports one chapter in every selected translation format.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token),
    fields(actor_user_id = %token.user_id),
)]
pub async fn export_translation<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    chapter_id: String,
    formats: ExportFormatSpec,
    with_raw_ident: bool,
) -> BaseRest<ExportChapterTranslationsVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + PageSourceRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    O: ObjDeptView<PageSourceImage, C> + Sync,
{
    let actor_has_artwork_assignment =
        chapter_port_perm_usecase::ensure_export_access::<C, R>(
            repo,
            &token,
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

    let page_source_infos = ListPageSourceInfos {
        chapter_id: &chapter_info.id,
    }
    .run_on(repo)
    .await?;

    let page_source_ids = page_source_infos
        .iter()
        .map(|page_source_info| page_source_info.id.as_str())
        .collect::<Vec<_>>();

    let unit_infos = ListUnitInfosByPageSourceIds {
        page_source_ids: &page_source_ids,
    }
    .run_on(repo)
    .await?;

    let raw_ident_infos = match with_raw_ident.then_some(()) {
        //
        Some(()) => {
            //
            ListPageSourceRawIdentInfos {
                page_source_ids: &page_source_ids,
            }
            .run_on(repo)
            .await?
        }

        None => Vec::new(),
    };

    let page_source_image_ids = PageSourceImage::ids(&page_source_ids)?;

    let obj_metas =
        ListObjMetas::<PageSourceImage>::new(&page_source_image_ids)
            .run_on(obj_dept)
            .await
            .map_err(BaseError::from)?;

    let snapshot = ChapterExportSnapshot {
        chapter_info: &chapter_info,
        comic_title: comic_info.title,
        page_source_infos: &page_source_infos,
        unit_infos,
        raw_ident_infos: &raw_ident_infos,
        obj_metas: &obj_metas,
    };

    let val = make_export_val(snapshot, formats, with_raw_ident);

    persist_export_record(
        (nucl, repo),
        &chapter_info.id,
        token.user_id,
        formats,
        actor_has_artwork_assignment,
    )
    .await?;

    accept!(val)
}

// Assembles requested exchange documents from the loaded export snapshot.
fn make_export_val(
    snapshot: ChapterExportSnapshot<'_>,
    formats: ExportFormatSpec,
    with_raw_ident: bool,
) -> ExportChapterTranslationsVal {
    //
    let ChapterExportSnapshot {
        chapter_info,
        comic_title,
        page_source_infos,
        unit_infos,
        raw_ident_infos,
        obj_metas,
    } = snapshot;

    let raw_ident_by_page_source_id = raw_ident_infos
        .iter()
        .map(|info| (info.page_source_id.clone(), info.raw_ident.clone()))
        .collect::<HashMap<_, _>>();

    let raw_idents = with_raw_ident.then(|| {
        //
        page_source_infos
            .iter()
            .filter_map(|page_source_info| {
                //
                raw_ident_by_page_source_id
                    .get(page_source_info.id.as_str())
                    .map(|raw_ident| ChapterPageSourceRawIdentVal {
                        page_source_id: page_source_info.id.clone(),
                        raw_ident: raw_ident.clone(),
                    })
            })
            .collect()
    });

    let mut units_by_page_source_id = HashMap::<String, Vec<UnitInfo>>::new();

    let mut ext_by_page_source_id = HashMap::<String, String>::new();

    for unit_info in unit_infos
        .into_iter()
        .filter(|unit_info| unit_info.hidden_at.is_none())
    {
        //
        units_by_page_source_id
            .entry(unit_info.page_source_id.clone())
            .or_default()
            .push(unit_info);
    }

    for page_source_info in page_source_infos {
        //
        if let Some(obj_meta) = obj_metas.get(page_source_info.id.as_str()) {
            //
            ext_by_page_source_id.insert(
                page_source_info.id.clone(),
                obj_meta.ext.suffix().to_owned(),
            );
        }
    }

    let poprako = formats.includes_poprako().then(|| {
        //
        make_native_document(
            chapter_info,
            comic_title,
            page_source_infos,
            &units_by_page_source_id,
        )
    });

    ExportChapterTranslationsVal {
        label_plus: formats.includes_label_plus().then(|| {
            //
            chapter_translation_export_complex::make_label_plus(
                page_source_infos,
                &units_by_page_source_id,
                &ext_by_page_source_id,
                &raw_ident_by_page_source_id,
            )
        }),
        poprako,
        raw_idents,
    }
}

// Persists a completed export and conditionally starts typesetting/redraw in one transaction.
async fn persist_export_record<N, C, R>(
    (nucl, repo): (&N, &R),
    chapter_id: &str,
    actor_user_id: String,
    formats: ExportFormatSpec,
    actor_has_artwork_assignment: bool,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + ChapterWorkflowRecordRepo<C> + Send + Sync,
{
    let () = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let entry = ChapterWorkflowRecordEntryShape::new(
                chapter_info.id.as_str(),
                Some(actor_user_id.as_str()),
                ChapterWorkflowRecordPayload::TranslationExported { formats },
            );

            CreateChapterWorkflowRecords {
                entries: std::slice::from_ref(&entry),
            }
            .step_on(repo, context)
            .await?;

            if actor_has_artwork_assignment {
                //
                stage_usecase::start_pending_stages(
                    repo,
                    context,
                    stage_usecase::PendingStageStart::new(
                        &chapter_info.id,
                        Some(&actor_user_id),
                        ChapterWorkflowRecordOrigin::TranslationExport,
                    ),
                    &[Stage::TypesetRedraw],
                )
                .await?;
            }

            accept!(())
        })
        .await?;

    accept!(())
}

// Renders the native document while preserving page and unit ordering.
fn make_native_document(
    chapter_info: &ChapterInfo,
    comic_title: String,
    page_source_infos: &[PageSourceInfo],
    units_by_page_source_id: &HashMap<String, Vec<UnitInfo>>,
) -> ChapterTranslationPortView {
    //
    let page_source_documents = page_source_infos
        .iter()
        .map(|page_source_info| {
            //
            let unit_documents = units_by_page_source_id
                .get(page_source_info.id.as_str())
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .enumerate()
                .map(|(index, unit_info)| {
                    make_unit_export(page_source_info, index, unit_info)
                })
                .collect();

            PageSourceTranslationDocument {
                page_source_id: page_source_info.id.clone(),
                page_source_index: page_source_info.index,
                units: unit_documents,
            }
        })
        .collect();

    let document = ChapterTranslationDocument {
        chapter_id: chapter_info.id.clone(),
        chapter_index: chapter_info.index,
        chapter_subtitle: non_empty(&chapter_info.subtitle),
        comic_id: chapter_info.comic_id.clone(),
        comic_title,
        pages: page_source_documents,
    };

    document.into()
}

// Builds a [`UnitTranslationDocument`] from page and unit info.
fn make_unit_export(
    page_source_info: &PageSourceInfo,
    index: usize,
    unit_info: &UnitInfo,
) -> UnitTranslationDocument {
    // Preserve unit content and authorship in the native exchange document.
    UnitTranslationDocument {
        unit_id: unit_info.id.clone(),
        unit_index: index,
        page_source_id: page_source_info.id.clone(),
        page_source_index: page_source_info.index,
        x_coord: unit_info.coord.x_coord,
        y_coord: unit_info.coord.y_coord,
        is_bubble: unit_info.is_bubble,
        is_flagged: unit_info.is_flagged,
        typography_note: unit_info.typography_note.clone(),
        translation_note: unit_info.translation_note.clone(),
        translated_text: unit_info.translated_text.clone(),
        translator_id: unit_info.last_translator_id.clone(),
        is_proofread: unit_info.is_proofread,
        proofread_text: unit_info.proofread_text.clone(),
        proofreader_id: unit_info.last_proofreader_id.clone(),
    }
}

// Returns [`Some`] with the text if non-empty, [`None`] otherwise.
fn non_empty(text: &str) -> Option<String> {
    //
    // Trim and normalize optional text fields before sending user-facing translation payloads.
    if text.trim().is_empty() {
        return None;
    }

    Some(text.to_string())
}
