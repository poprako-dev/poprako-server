//! Chapter page-manifest and page-image allocation.

/// Manifest validation rules.
pub mod validation;

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::{ObjSlot, ObjSlotSpec};
use poprako_obj_dept::oper::{
    DeleteObjs, GenObjSlot, GenObjSlots, ListObjMetas,
};
use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex, manifest};
use crate::config::image::ImageConfig;
use crate::data::instr::page::{AllocChapterPagesInstr, AllocPageImageInstr};
use crate::data::val::page::{AllocChapterPagesVal, AllocatedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageImageSpec, PageManifestEntry};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageImage;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::{
    ApplyPageManifest, DeletePages, GetPageInfo, GetPageInfoExcluded,
    ListPageInfosExcluded, ShiftPageIndexesTemporary,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, ImageKind, PageImageKey};

// One page and its resolved object allocation.
struct PageAlloc {
    //
    // Stable page identifier.
    page_id: String,
    // Final page position.
    index: usize,
    // Expected content hash.
    image_hash: ImageHash,
    // Expected image suffix.
    ext: ImageExt,
    // Optional new upload capability.
    obj_slot: Option<ObjSlot>,
}

/// Allocates the authoritative page manifest and its required image uploads.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_chapter_pages<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    instr: AllocChapterPagesInstr,
) -> BaseRest<AllocChapterPagesVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let AllocChapterPagesInstr { chapter_id, pages } = instr;

    let page_specs = pages
        .into_iter()
        .map(PageImageSpec::from)
        .collect::<Vec<_>>();

    let page_count = validation::validate_page_specs(
        image_config,
        &page_specs,
        &chapter_id,
        &token.user_id,
    )?;

    ensure_alloc_perm::<C, R>(repo, &token, &chapter_id).await?;

    let allocs = nucl
        .coord(async move |context| {
            //
            apply_manifest(
                (repo, prom, obj_dept, context),
                &token.user_id,
                &chapter_id,
                &page_specs,
                page_count,
            )
            .await
        })
        .await?;

    let pages = allocs
        .into_iter()
        .map(alloc_val)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(AllocChapterPagesVal { pages })
}

/// Allocates a replacement image generation for one page.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config, token), fields(actor_user_id = %token.user_id))]
pub async fn alloc_image<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: AllocPageImageInstr,
) -> BaseRest<AllocatedPageVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    ImageComplex::ensure_byte_length(
        image_config,
        instr.new_byte_len,
        ImageKind::PageImage,
    )?;

    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    ensure_alloc_perm::<C, R>(repo, &token, &page_info.chapter_id).await?;

    let page_id = id.clone();

    let page_index = page_info.index;

    let image_hash = instr.image_hash.clone();

    let image_ext = instr.ext;

    let obj_slot = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            GetPageInfoExcluded { id: &id }
                .step_on(repo, context)
                .await?;

            let obj_spec = ObjSlotSpec {
                dom: PageImageKey {
                    chapter_id: page_info.chapter_id.clone(),
                    page_id: id.clone(),
                    ext: instr.ext,
                },
                hash: instr.image_hash.as_bytes(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            let obj_slot = GenObjSlot::<PageImage>::new(&obj_spec)
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

            if obj_slot.is_some() {
                //
                let advance_id = next_snowflake_id();

                let advance_payload = TaskPayload::Chapter {
                    payload: ChapterPayload::TryAdvanceRawProvideStage {
                        chapter_id: page_info.chapter_id.clone(),
                        actor_user_id: Some(token.user_id.clone()),
                    },
                };

                let advance_task = Task {
                    id: &advance_id,
                    payload: &advance_payload,
                    delay: Some(Duration::from_mins(20)),
                };

                Defer::new(advance_task).step_on(prom, context).await?;
            }

            accept(obj_slot)
        })
        .await?;

    alloc_val(PageAlloc {
        page_id,
        index: page_index,
        image_hash,
        ext: image_ext,
        obj_slot,
    })
}

// Applies the manifest diff and object obligations in one transaction.
#[expect(clippy::too_many_lines, reason = "coordinates manifest invariants")]
async fn apply_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_specs: &[PageImageSpec],
    page_count: usize,
) -> BaseRest<Vec<PageAlloc>>
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

    ChapterComplex::ensure_chapter_writable(&chapter_info)?;

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

    let manifest_candidates = existing_page_infos
        .iter()
        .map(|page_info| {
            //
            let obj_meta = existing_obj_metas.get(&page_info.id);

            manifest::PageManifestCand {
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

    let manifest_plan = manifest::PageManifestComplex::build(
        &chapter_info.id,
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
            .ok_or_else(page_manifest_result_missing)?;

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
                .ok_or_else(page_manifest_result_missing)
        })
        .collect::<BaseRest<Vec<_>>>()?;

    let manifest_entries = manifest_plan
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
                    .ok_or_else(page_manifest_result_missing)?,

                None => PageComplex::gen_id(),
            };

            accept(PageManifestEntry {
                id,
                chapter_id: chapter_info.id.clone(),
                index,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    ShiftPageIndexesTemporary {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let page_infos = ApplyPageManifest {
        entries: &manifest_entries,
    }
    .step_on(repo, context)
    .await?;

    let page_infos_by_id = page_infos
        .iter()
        .map(|page_info| (page_info.id.as_str(), page_info))
        .collect::<HashMap<_, _>>();

    if page_infos_by_id.len() != page_count {
        return Err(page_manifest_result_missing());
    }

    let retained_page_ids = manifest_entries
        .iter()
        .zip(page_specs)
        .filter(|(_, page_spec)| page_spec.new_byte_len.is_none())
        .map(|(manifest_entry, _)| manifest_entry.id.as_str())
        .collect::<Vec<_>>();

    let retained_obj_metas = ListObjMetas::<PageImage>::new(&retained_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    for (manifest_entry, page_spec) in manifest_entries.iter().zip(page_specs) {
        //
        let None = page_spec.new_byte_len else {
            continue;
        };

        let page_info = page_infos_by_id
            .get(manifest_entry.id.as_str())
            .ok_or_else(page_manifest_result_missing)?;

        ensure_retained_obj(
            page_info,
            page_spec,
            retained_obj_metas.get(&manifest_entry.id),
        )?;
    }

    let obj_specs = manifest_entries
        .iter()
        .zip(page_specs)
        .filter_map(|(manifest_entry, page_spec)| {
            //
            let byte_len = page_spec.new_byte_len?;

            Some(ObjSlotSpec {
                dom: PageImageKey {
                    chapter_id: chapter_info.id.clone(),
                    page_id: manifest_entry.id.clone(),
                    ext: page_spec.ext,
                },
                hash: page_spec.image_hash.as_bytes(),
                content_type: page_spec.ext.content_type(),
                byte_len,
            })
        })
        .collect::<Vec<_>>();

    let mut obj_slots = GenObjSlots::<PageImage>::new(&obj_specs)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let page_allocs = manifest_entries
        .iter()
        .zip(page_specs)
        .map(|(manifest_entry, page_spec)| {
            //
            let page_info = page_infos_by_id
                .get(manifest_entry.id.as_str())
                .ok_or_else(page_manifest_result_missing)?;

            let obj_slot = match page_spec.new_byte_len {
                //
                Some(_) => obj_slots.remove(&manifest_entry.id),

                None => None,
            };

            accept(PageAlloc {
                page_id: page_info.id.clone(),
                index: page_info.index,
                image_hash: page_spec.image_hash.clone(),
                ext: page_spec.ext,
                obj_slot,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    DeleteObjs::<PageImage>::new(&deleted_page_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    DeletePages::Ids {
        ids: &deleted_page_ids,
    }
    .step_on(repo, context)
    .await?;

    let (total_unit_count, translated_unit_count, proofread_unit_count) =
        page_counters(&existing_page_infos, &retained_ids);

    SetChapterPageCounters {
        id: &chapter_info.id,
        page_count,
        total_unit_count,
        translated_unit_count,
        proofread_unit_count,
    }
    .step_on(repo, context)
    .await?;

    let advance_id = next_snowflake_id();

    let advance_payload = TaskPayload::Chapter {
        payload: ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id: chapter_info.id.clone(),
            actor_user_id: Some(user_id.to_owned()),
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

    accept(page_allocs)
}

// Converts one internal allocation into its response view.
fn alloc_val(alloc: PageAlloc) -> BaseRest<AllocatedPageVal> {
    //
    let slot = alloc.obj_slot.map(|obj_slot| ImageUploadSlotView {
        put_url: obj_slot.url.to_string(),
        image_ver: obj_slot.key.ver,
        headers: obj_slot.headers,
    });

    let index = u32::try_from(alloc.index).map_err(|_| {
        //
        BaseError::Unrecoverable {
            message: "page index is out of range".into(),
        }
    })?;

    accept(AllocatedPageVal {
        page_id: alloc.page_id,
        index,
        image_hash: alloc.image_hash,
        ext: alloc.ext,
        slot,
    })
}

// Builds an internal error for an incomplete page manifest result.
fn page_manifest_result_missing() -> BaseError {
    //
    BaseError::Unrecoverable {
        message: "page manifest result is incomplete".into(),
    }
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
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id),
        });
    };

    let same_hash = obj_meta.hash.as_slice() == page_spec.image_hash.as_bytes();

    match (same_hash, obj_meta.ext == page_spec.ext.suffix()) {
        //
        (true, true) => accept(()),

        _ => Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id),
        }),
    }
}

// Sums unit counters retained by the new manifest.
fn page_counters(
    page_infos: &[PageInfo],
    retained_ids: &HashSet<&str>,
) -> (usize, usize, usize) {
    //
    page_infos
        .iter()
        .filter(|page_info| retained_ids.contains(page_info.id.as_str()))
        .fold((0, 0, 0), |counters, page_info| {
            //
            (
                counters.0 + page_info.total_unit_count,
                counters.1 + page_info.translated_unit_count,
                counters.2 + page_info.proofread_unit_count,
            )
        })
}

// Validates the caller and current chapter state.
async fn ensure_alloc_perm<C, R>(
    repo: &R,
    token: &UserToken,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-page-alloc-role-required"),
        });
    };

    PagePermComplex::ensure_user_can_alloc(&assignment_info)
}
