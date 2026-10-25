//! Authoritative chapter page manifest transactions.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use poprako_orchestra::{Context, OperStep as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::model::alloc::ObjAlloc;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::oper::{DeleteObjs, GenObjSlots, ListObjMetas};

use crate::complex::page_source::manifest as page_source_manifest_complex;
use crate::complex::{
    chapter as chapter_complex, page_source as page_source_complex,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::write::page_source::{
    PageSourceImageSpec, PageSourceManifestEntry,
};
use crate::part::obj_dept::PageSourceImage;
use crate::part::prom::Prom;
use crate::part::prom::payload::PromPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageSourceCountMetrics,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    ApplyPageSourceManifest, DeletePageSources, ListPageSourceInfosExcluded,
    ShiftPageSourceIndexesTemporary, UpdatePageSourceRawIdents,
};
use crate::part::repo::page_source::PageSourceRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::page_source::PageSourceRawIdentsReplShape;
use crate::usecase::page_source::alloc::permission::ensure_alloc_perm;
use crate::util::next_snowflake_id;
use crate::value::business_id::{ChapterId, PageSourceId};
use crate::value::image::{ImageExt, ImageHash, PageSourceImageKey};
use crate::value::page_source::PageSourceManifestCand;

// Borrows prior counters and matched identities until the final aggregate refresh.
#[derive(Clone, Copy)]
struct PageSourceManifestCountScope<'a> {
    // Prior counters contribute only when their page identity survives matching.
    existing_page_source_infos: &'a [PageSourceInfo],
    // PageSource identities retained by the authoritative manifest.
    retained_ids: &'a HashSet<&'a str>,

    // Authoritative number of pages requested by this manifest.
    page_source_count: usize,
}

/// Applies the manifest diff and object obligations in one transaction.
pub async fn apply_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_source_specs: &[PageSourceImageSpec],
    page_source_count: usize,
) -> BaseRest<
    Vec<(
        String,
        usize,
        ImageHash,
        ImageExt,
        ObjAlloc<PageSourceImage>,
    )>,
>
where
    C: Context + Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + PageSourceRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    chapter_complex::ensure_chapter_writable(&chapter_info)?;

    ensure_alloc_perm(repo, context, user_id, chapter_id).await?;

    let existing_page_source_infos = ListPageSourceInfosExcluded {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let (entries, retained_ids, deleted_page_source_ids) = plan_manifest(
        &chapter_info.id,
        &existing_page_source_infos,
        page_source_specs,
    )?;

    let page_source_infos = persist_manifest_entries(
        (repo, context),
        &chapter_info.id,
        &entries,
        page_source_specs,
    )
    .await?;

    let page_source_allocs = allocate_manifest_objs(
        (obj_dept, context),
        &chapter_info.id,
        (&entries, page_source_specs),
        &page_source_infos,
        page_source_count,
    )
    .await?;

    let count_scope = PageSourceManifestCountScope {
        existing_page_source_infos: &existing_page_source_infos,
        retained_ids: &retained_ids,
        page_source_count,
    };

    finish_manifest(
        (repo, prom, obj_dept, context),
        &chapter_info,
        count_scope,
        &deleted_page_source_ids,
        user_id,
    )
    .await?;

    accept!(page_source_allocs)
}

// Matches the loaded snapshot and preserves identities before any mutation.
fn plan_manifest<'a>(
    chapter_id: &str,
    existing_page_source_infos: &'a [PageSourceInfo],
    page_source_specs: &[PageSourceImageSpec],
) -> BaseRest<(Vec<PageSourceManifestEntry>, HashSet<&'a str>, Vec<String>)> {
    //
    let manifest_candidates = existing_page_source_infos
        .iter()
        .map(|page_source_info| PageSourceManifestCand {
            id: &page_source_info.id,
            chapter_id: &page_source_info.chapter_id,
        })
        .collect::<Vec<_>>();

    let manifest_plan = page_source_manifest_complex::build(
        chapter_id,
        &manifest_candidates,
        page_source_specs,
    )?;

    let mut retained_ids = HashSet::new();

    for manifest_match in &manifest_plan.matches {
        //
        let Some(existing_index) = manifest_match.existing_index else {
            continue;
        };

        let page_source_info = existing_page_source_infos
            .get(existing_index)
            .ok_or_else(page_source_manifest_incomplete)?;

        retained_ids.insert(page_source_info.id.as_str());
    }

    let deleted_page_source_ids = manifest_plan
        .deleted_existing_indexes
        .iter()
        .map(|existing_index| {
            //
            existing_page_source_infos
                .get(*existing_index)
                .map(|page_source_info| page_source_info.id.clone())
                .ok_or_else(page_source_manifest_incomplete)
        })
        .collect::<BaseRest<Vec<_>>>()?;

    let entries = manifest_plan
        .matches
        .iter()
        .enumerate()
        .map(|(index, manifest_match)| {
            //
            let id = match manifest_match.existing_index {
                //
                Some(existing_index) => existing_page_source_infos
                    .get(existing_index)
                    .map(|page_source_info| page_source_info.id.clone())
                    .ok_or_else(page_source_manifest_incomplete)?,

                None => page_source_complex::gen_id(),
            };

            accept!(PageSourceManifestEntry {
                id,
                chapter_id: chapter_id.to_owned(),
                index,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept!((entries, retained_ids, deleted_page_source_ids))
}

// Reorders pages and writes filename associations in the caller's transaction.
async fn persist_manifest_entries<C, R>(
    (repo, context): (&R, &mut C),
    chapter_id: &str,
    entries: &[PageSourceManifestEntry],
    page_source_specs: &[PageSourceImageSpec],
) -> BaseRest<Vec<PageSourceInfo>>
where
    C: Context + Send,
    R: PageSourceRepo<C> + Sync,
{
    //
    ShiftPageSourceIndexesTemporary { chapter_id }
        .step_on(repo, context)
        .await?;

    let page_source_infos = ApplyPageSourceManifest { entries }
        .step_on(repo, context)
        .await?;

    let raw_ident_repls = entries
        .iter()
        .zip(page_source_specs)
        .map(|(entry, spec)| (entry.id.as_str(), spec.raw_ident.as_deref()))
        .collect::<Vec<_>>();

    let raw_ident_repl = PageSourceRawIdentsReplShape {
        idents: raw_ident_repls,
    };

    UpdatePageSourceRawIdents {
        repl: &raw_ident_repl,
    }
    .step_on(repo, context)
    .await?;

    accept!(page_source_infos)
}

// Checks persisted pages and allocates bytes for position-aligned entries and specs.
async fn allocate_manifest_objs<C, O>(
    (obj_dept, context): (&O, &mut C),
    chapter_id: &str,
    (entries, page_source_specs): (
        &[PageSourceManifestEntry],
        &[PageSourceImageSpec],
    ),
    page_source_infos: &[PageSourceInfo],
    page_source_count: usize,
) -> BaseRest<
    Vec<(
        String,
        usize,
        ImageHash,
        ImageExt,
        ObjAlloc<PageSourceImage>,
    )>,
>
where
    C: Context + Send,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    //
    let page_source_infos_by_id = page_source_infos
        .iter()
        .map(|page_source_info| {
            (page_source_info.id.as_str(), page_source_info)
        })
        .collect::<HashMap<_, _>>();

    if page_source_infos_by_id.len() != page_source_count {
        return Err(page_source_manifest_incomplete());
    }

    let retained_obj_metas = validate_retained_objs(
        (obj_dept, context),
        entries,
        page_source_specs,
        &page_source_infos_by_id,
    )
    .await?;

    let obj_specs = entries
        .iter()
        .zip(page_source_specs)
        .filter(|(_, page_source_spec)| page_source_spec.new_byte_len.is_some())
        .map(|(entry, page_source_spec)| {
            //
            let byte_len = page_source_spec
                .new_byte_len
                .ok_or_else(page_source_manifest_incomplete)?;

            Ok(ObjSlotSpec {
                dom: PageSourceImageKey {
                    chapter_id: ChapterId::new(chapter_id)?,
                    page_source_id: PageSourceId::new(&entry.id)?,
                    ext: page_source_spec.ext,
                },
                hash: page_source_spec.image_hash.as_bytes(),
                content_type: page_source_spec.ext.content_type(),
                byte_len,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    let mut obj_allocs = GenObjSlots::<PageSourceImage>::new(&obj_specs)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let page_source_allocs = entries
        .iter()
        .zip(page_source_specs)
        .map(|(entry, page_source_spec)| {
            //
            let page_source_info = page_source_infos_by_id
                .get(entry.id.as_str())
                .ok_or_else(page_source_manifest_incomplete)?;

            let obj_slot = match page_source_spec.new_byte_len {
                //
                Some(_) => obj_allocs
                    .remove(entry.id.as_str())
                    .ok_or_else(page_source_manifest_incomplete)?,

                None => ObjAlloc::Avail {
                    key: retained_obj_metas
                        .get(entry.id.as_str())
                        .ok_or_else(page_source_manifest_incomplete)?
                        .key
                        .clone(),
                },
            };

            accept!((
                page_source_info.id.clone(),
                page_source_info.index,
                page_source_spec.image_hash.clone(),
                page_source_spec.ext,
                obj_slot,
            ))
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept!(page_source_allocs)
}

// Deletes obsolete pages, refreshes counters, and schedules stage advancement atomically.
async fn finish_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    chapter_info: &ChapterInfo,
    count_scope: PageSourceManifestCountScope<'_>,
    deleted_page_source_ids: &[String],
    user_id: &str,
) -> BaseRest<()>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + PageSourceRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    //
    let PageSourceManifestCountScope {
        existing_page_source_infos,
        retained_ids,
        page_source_count,
    } = count_scope;

    let page_source_image_ids = PageSourceImage::ids(deleted_page_source_ids)?;

    DeleteObjs::<PageSourceImage>::new(&page_source_image_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    DeletePageSources::Ids {
        ids: deleted_page_source_ids,
    }
    .step_on(repo, context)
    .await?;

    let (total_unit_count, translated_unit_count, proofread_unit_count) =
        page_source_count_metrics(existing_page_source_infos, retained_ids);

    SetChapterPageSourceCountMetrics {
        id: &chapter_info.id,
        page_source_count,
        total_unit_count,
        translated_unit_count,
        proofread_unit_count,
    }
    .step_on(repo, context)
    .await?;

    let advance_id = next_snowflake_id();

    let advance_payload = PromPayload::Chapter {
        payload: ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id: chapter_info.id.clone(),
            actor_user_id: user_id.to_owned(),
        },
    };

    let advance_task = Task {
        id: &advance_id,
        payload: &advance_payload,
        delay: Some(Duration::from_mins(20)),
    };

    Defer::new(advance_task).step_on(prom, context).await?;

    TouchComicLastActive {
        id: &chapter_info.comic_id,
    }
    .step_on(repo, context)
    .await?;

    accept!(())
}

// Builds an internal error for an incomplete page manifest result.
fn page_source_manifest_incomplete() -> BaseError {
    //
    unrecoverable_error! {
        message: "page manifest result is incomplete".into();
        "page manifest result is incomplete",
    }
}

// Verifies retained page objects against the requested immutable image identity.
async fn validate_retained_objs<C, O>(
    (obj_dept, context): (&O, &mut C),
    entries: &[PageSourceManifestEntry],
    page_source_specs: &[PageSourceImageSpec],
    page_source_infos_by_id: &HashMap<&str, &PageSourceInfo>,
) -> BaseRest<HashMap<PageSourceId, ObjMeta<PageSourceImage>>>
where
    C: Context + Send,
    O: ObjDept<PageSourceImage, C> + Send + Sync,
{
    //
    let retained_page_source_ids = entries
        .iter()
        .zip(page_source_specs)
        .filter(|(_, page_source_spec)| page_source_spec.new_byte_len.is_none())
        .map(|(entry, _)| entry.id.as_str())
        .collect::<Vec<_>>();

    let page_source_image_ids =
        PageSourceImage::ids(&retained_page_source_ids)?;

    let retained_obj_metas =
        ListObjMetas::<PageSourceImage>::new(&page_source_image_ids)
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

    for (entry, page_source_spec) in entries.iter().zip(page_source_specs) {
        //
        let None = page_source_spec.new_byte_len else {
            continue;
        };

        let page_source_info = page_source_infos_by_id
            .get(entry.id.as_str())
            .ok_or_else(page_source_manifest_incomplete)?;

        ensure_retained_obj(
            page_source_info,
            page_source_spec,
            retained_obj_metas.get(entry.id.as_str()),
        )?;
    }

    accept!(retained_obj_metas)
}

// Sums unit counters retained by the new manifest.
fn page_source_count_metrics(
    page_source_infos: &[PageSourceInfo],
    retained_ids: &HashSet<&str>,
) -> (usize, usize, usize) {
    //
    page_source_infos
        .iter()
        .filter(|page_source_info| {
            retained_ids.contains(page_source_info.id.as_str())
        })
        .fold((0, 0, 0), |count_metrics, page_source_info| {
            //
            (
                count_metrics.0 + page_source_info.total_unit_count,
                count_metrics.1 + page_source_info.translated_unit_count,
                count_metrics.2 + page_source_info.proofread_unit_count,
            )
        })
}

// Verifies that a retained page still points at the requested bytes.
fn ensure_retained_obj(
    page_source_info: &PageSourceInfo,
    page_source_spec: &PageSourceImageSpec,
    obj_meta: Option<&ObjMeta<PageSourceImage>>,
) -> BaseRest<()> {
    //
    let Some(obj_meta) = obj_meta.filter(|meta| meta.is_avail) else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: format!(
                "page {} requires a new upload",
                page_source_info.id,
            );
            page_source_id = %page_source_info.id,
            "page requires a new upload",
        });
    };

    let same_hash =
        obj_meta.hash.as_slice() == page_source_spec.image_hash.as_bytes();

    match (same_hash, obj_meta.ext == page_source_spec.ext) {
        //
        (true, true) => accept!(()),

        _ => Err(expected_error! {
            ExpectedVariant::Args,
            message: format!(
                "page {} requires a new upload",
                page_source_info.id,
            );
            page_source_id = %page_source_info.id,
            "page requires a new upload",
        }),
    }
}
