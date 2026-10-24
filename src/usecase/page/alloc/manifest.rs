//! Authoritative chapter page manifest transactions.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use poprako_orchestra::{Context, OperStep as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::{ObjSlot, ObjSlotSpec};
use poprako_obj_dept::oper::{DeleteObjs, GenObjSlots, ListObjMetas};

use crate::complex::page::manifest as page_manifest_complex;
use crate::complex::{chapter as chapter_complex, page as page_complex};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::{PageImageSpec, PageManifestEntry};
use crate::part::obj_dept::PageImage;
use crate::part::prom::Prom;
use crate::part::prom::payload::PromPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCountMetrics,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, ListPageInfosExcluded,
    ShiftPageIndexesTemporary, UpdatePageRawIdents,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::page::PageRawIdentsReplShape;
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, PageImageKey};

// Borrows prior counters and matched identities until the final aggregate refresh.
#[derive(Clone, Copy)]
struct PageManifestCountScope<'a> {
    // Prior counters contribute only when their page identity survives matching.
    existing_page_infos: &'a [PageInfo],
    // Page identities retained by the authoritative manifest.
    retained_ids: &'a HashSet<&'a str>,

    // Authoritative number of pages requested by this manifest.
    page_count: usize,
}

/// Applies the manifest diff and object obligations in one transaction.
pub async fn apply_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_specs: &[PageImageSpec],
    page_count: usize,
) -> BaseRest<Vec<(String, usize, ImageHash, ImageExt, Option<ObjSlot>)>>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    chapter_complex::ensure_chapter_writable(&chapter_info)?;

    let existing_page_infos = ListPageInfosExcluded {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let existing_page_ids = existing_page_infos
        .iter()
        .map(|page_info| page_info.id.as_str())
        .collect::<Vec<_>>();

    let existing_obj_metas = ListObjMetas::<PageImage>::new(&existing_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let (entries, retained_ids, deleted_page_ids) = plan_manifest(
        &chapter_info.id,
        &existing_page_infos,
        &existing_obj_metas,
        page_specs,
    )?;

    let page_infos = persist_manifest_entries(
        (repo, context),
        &chapter_info.id,
        &entries,
        page_specs,
    )
    .await?;

    let page_allocs = allocate_manifest_objs(
        (obj_dept, context),
        &chapter_info.id,
        (&entries, page_specs),
        &page_infos,
        page_count,
    )
    .await?;

    let count_scope = PageManifestCountScope {
        existing_page_infos: &existing_page_infos,
        retained_ids: &retained_ids,
        page_count,
    };

    finish_manifest(
        (repo, prom, obj_dept, context),
        &chapter_info,
        count_scope,
        &deleted_page_ids,
        user_id,
    )
    .await?;

    accept!(page_allocs)
}

// Matches the loaded snapshot and preserves identities before any mutation.
fn plan_manifest<'a>(
    chapter_id: &str,
    existing_page_infos: &'a [PageInfo],
    existing_obj_metas: &HashMap<String, ObjMeta>,
    page_specs: &[PageImageSpec],
) -> BaseRest<(Vec<PageManifestEntry>, HashSet<&'a str>, Vec<String>)> {
    //
    let manifest_candidates = existing_page_infos
        .iter()
        .map(|page_info| {
            //
            let obj_meta = existing_obj_metas.get(&page_info.id);

            page_manifest_complex::PageManifestCand {
                id: &page_info.id,
                chapter_id: &page_info.chapter_id,
                index: page_info.index,
                has_units: page_info.total_unit_count > 0,
                image_uploaded: obj_meta.is_some_and(|meta| meta.is_avail),
                image_hash: obj_meta.map(|meta| meta.hash.as_slice()),
                image_ext: obj_meta.map(|meta| meta.ext.as_str()),
            }
        })
        .collect::<Vec<_>>();

    let manifest_plan = page_manifest_complex::build(
        chapter_id,
        &manifest_candidates,
        page_specs,
    )?;

    let mut retained_ids = HashSet::new();

    for manifest_match in &manifest_plan.matches {
        //
        let Some(existing_index) = manifest_match.existing_index else {
            continue;
        };

        let page_info = existing_page_infos
            .get(existing_index)
            .ok_or_else(page_manifest_incomplete)?;

        retained_ids.insert(page_info.id.as_str());
    }

    let deleted_page_ids = manifest_plan
        .deleted_existing_indexes
        .iter()
        .map(|existing_index| {
            //
            existing_page_infos
                .get(*existing_index)
                .map(|page_info| page_info.id.clone())
                .ok_or_else(page_manifest_incomplete)
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
                Some(existing_index) => existing_page_infos
                    .get(existing_index)
                    .map(|page_info| page_info.id.clone())
                    .ok_or_else(page_manifest_incomplete)?,

                None => page_complex::gen_id(),
            };

            accept!(PageManifestEntry {
                id,
                chapter_id: chapter_id.to_owned(),
                index,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept!((entries, retained_ids, deleted_page_ids))
}

// Reorders pages and writes filename associations in the caller's transaction.
async fn persist_manifest_entries<C, R>(
    (repo, context): (&R, &mut C),
    chapter_id: &str,
    entries: &[PageManifestEntry],
    page_specs: &[PageImageSpec],
) -> BaseRest<Vec<PageInfo>>
where
    C: Context + Send,
    R: PageRepo<C> + Sync,
{
    //
    ShiftPageIndexesTemporary { chapter_id }
        .step_on(repo, context)
        .await?;

    let page_infos =
        ApplyPageManifest { entries }.step_on(repo, context).await?;

    let raw_ident_repls = entries
        .iter()
        .zip(page_specs)
        .map(|(entry, spec)| (entry.id.as_str(), spec.raw_ident.as_deref()))
        .collect::<Vec<_>>();

    let raw_ident_repl = PageRawIdentsReplShape {
        idents: raw_ident_repls,
    };

    UpdatePageRawIdents {
        repl: &raw_ident_repl,
    }
    .step_on(repo, context)
    .await?;

    accept!(page_infos)
}

// Checks persisted pages and allocates bytes for position-aligned entries and specs.
async fn allocate_manifest_objs<C, O>(
    (obj_dept, context): (&O, &mut C),
    chapter_id: &str,
    (entries, page_specs): (&[PageManifestEntry], &[PageImageSpec]),
    page_infos: &[PageInfo],
    page_count: usize,
) -> BaseRest<Vec<(String, usize, ImageHash, ImageExt, Option<ObjSlot>)>>
where
    C: Context + Send,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    //
    let page_infos_by_id = page_infos
        .iter()
        .map(|page_info| (page_info.id.as_str(), page_info))
        .collect::<HashMap<_, _>>();

    if page_infos_by_id.len() != page_count {
        return Err(page_manifest_incomplete());
    }

    validate_retained_objs(
        (obj_dept, context),
        entries,
        page_specs,
        &page_infos_by_id,
    )
    .await?;

    let obj_specs = entries
        .iter()
        .zip(page_specs)
        .filter_map(|(entry, page_spec)| {
            //
            let byte_len = page_spec.new_byte_len?;

            Some(ObjSlotSpec {
                dom: PageImageKey {
                    chapter_id: chapter_id.to_owned(),
                    page_id: entry.id.clone(),
                    ext: page_spec.ext,
                },
                hash: page_spec.image_hash.as_bytes(),
                content_type: page_spec.ext.content_type(),
                byte_len,
            })
        })
        .collect::<Vec<_>>();

    let mut obj_allocs = GenObjSlots::<PageImage>::new(&obj_specs)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let page_allocs = entries
        .iter()
        .zip(page_specs)
        .map(|(entry, page_spec)| {
            //
            let page_info = page_infos_by_id
                .get(entry.id.as_str())
                .ok_or_else(page_manifest_incomplete)?;

            let obj_slot = match page_spec.new_byte_len {
                //
                Some(_) => obj_allocs
                    .remove(&entry.id)
                    .ok_or_else(page_manifest_incomplete)?
                    .into_slot(),

                None => None,
            };

            accept!((
                page_info.id.clone(),
                page_info.index,
                page_spec.image_hash.clone(),
                page_spec.ext,
                obj_slot,
            ))
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept!(page_allocs)
}

// Deletes obsolete pages, refreshes counters, and schedules stage advancement atomically.
async fn finish_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    chapter_info: &ChapterInfo,
    count_scope: PageManifestCountScope<'_>,
    deleted_page_ids: &[String],
    user_id: &str,
) -> BaseRest<()>
where
    C: Context + Send,
    R: ChapterRepo<C> + ComicRepo<C> + PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    //
    let PageManifestCountScope {
        existing_page_infos,
        retained_ids,
        page_count,
    } = count_scope;

    DeleteObjs::<PageImage>::new(deleted_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    DeletePages::Ids {
        ids: deleted_page_ids,
    }
    .step_on(repo, context)
    .await?;

    let (total_unit_count, translated_unit_count, proofread_unit_count) =
        page_count_metrics(existing_page_infos, retained_ids);

    SetChapterPageCountMetrics {
        id: &chapter_info.id,
        page_count,
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
fn page_manifest_incomplete() -> BaseError {
    //
    unrecoverable_error! {
        message: "page manifest result is incomplete".into();
        "page manifest result is incomplete",
    }
}

// Verifies retained page objects against the requested immutable image identity.
async fn validate_retained_objs<C, O>(
    (obj_dept, context): (&O, &mut C),
    entries: &[PageManifestEntry],
    page_specs: &[PageImageSpec],
    page_infos_by_id: &HashMap<&str, &PageInfo>,
) -> BaseRest<()>
where
    C: Context + Send,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    //
    let retained_page_ids = entries
        .iter()
        .zip(page_specs)
        .filter(|(_, page_spec)| page_spec.new_byte_len.is_none())
        .map(|(entry, _)| entry.id.as_str())
        .collect::<Vec<_>>();

    let retained_obj_metas = ListObjMetas::<PageImage>::new(&retained_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    for (entry, page_spec) in entries.iter().zip(page_specs) {
        //
        let None = page_spec.new_byte_len else {
            continue;
        };

        let page_info = page_infos_by_id
            .get(entry.id.as_str())
            .ok_or_else(page_manifest_incomplete)?;

        ensure_retained_obj(
            page_info,
            page_spec,
            retained_obj_metas.get(&entry.id),
        )?;
    }

    accept!(())
}

// Sums unit counters retained by the new manifest.
fn page_count_metrics(
    page_infos: &[PageInfo],
    retained_ids: &HashSet<&str>,
) -> (usize, usize, usize) {
    //
    page_infos
        .iter()
        .filter(|page_info| retained_ids.contains(page_info.id.as_str()))
        .fold((0, 0, 0), |count_metrics, page_info| {
            //
            (
                count_metrics.0 + page_info.total_unit_count,
                count_metrics.1 + page_info.translated_unit_count,
                count_metrics.2 + page_info.proofread_unit_count,
            )
        })
}

// Verifies that a retained page still points at the requested bytes.
fn ensure_retained_obj(
    page_info: &PageInfo,
    page_spec: &PageImageSpec,
    obj_meta: Option<&ObjMeta>,
) -> BaseRest<()> {
    //
    let Some(obj_meta) = obj_meta else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id);
            page_id = %page_info.id,
            "page requires a new upload",
        });
    };

    let same_hash = obj_meta.hash.as_slice() == page_spec.image_hash.as_bytes();

    match (same_hash, obj_meta.ext == page_spec.ext.suffix()) {
        //
        (true, true) => accept!(()),

        _ => Err(expected_error! {
            ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id);
            page_id = %page_info.id,
            "page requires a new upload",
        }),
    }
}
