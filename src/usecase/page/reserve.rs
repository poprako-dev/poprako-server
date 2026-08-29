//! Chapter page-manifest and page-image reservation.

/// Manifest validation rules.
pub mod validation;

use std::collections::HashSet;
use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::slot::{ObjSlot, ObjSlotSpec};
use poprako_obj_dept::{ObjDept, obj_inst};
use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::config::image::ImageConfig;
use crate::data::instr::page::{
    ReserveChapterPagesInstr, ReservePageImageInstr,
};
use crate::data::val::page::{ReserveChapterPagesVal, ReservedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageEntry, PageImageSpec, PageManifestRepl};
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
    CreatePages, DeletePages, GetPageInfo, GetPageInfoExcluded,
    ListPageInfosExcluded, ShiftPageIndexesTemporary, UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, ImageKind};

// One page and its resolved object reservation.
struct PageReservation {
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

/// Reserves the authoritative page manifest and its required image uploads.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config))]
pub async fn reserve_chapter_pages<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    instr: ReserveChapterPagesInstr,
) -> BaseRest<ReserveChapterPagesVal>
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
    let ReserveChapterPagesInstr { chapter_id, pages } = instr;

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

    ensure_reserve_perm::<C, R>(repo, &token, &chapter_id).await?;

    let reservations = nucl
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

    let pages = reservations
        .into_iter()
        .map(reservation_val)
        .collect::<BaseRest<Vec<_>>>()?;

    accept(ReserveChapterPagesVal { pages })
}

/// Reserves a replacement image generation for one page.
#[instrument(level = "info", skip(nucl, repo, prom, obj_dept, image_config))]
pub async fn reserve_image<N, C, R, P, O>(
    (nucl, repo, prom, obj_dept, image_config): (&N, &R, &P, &O, &ImageConfig),
    token: UserToken,
    id: String,
    instr: ReservePageImageInstr,
) -> BaseRest<ReservedPageVal>
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

    ensure_reserve_perm::<C, R>(repo, &token, &page_info.chapter_id).await?;

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
                id: &id,
                hash: instr.image_hash.as_bytes(),
                ext: instr.ext.suffix(),
                content_type: instr.ext.content_type(),
                byte_len: instr.new_byte_len,
            };

            let obj_slot =
                obj_inst! { GenObjSlot<PageImage> { spec: &obj_spec } }
                    .step_on(obj_dept, context)
                    .await
                    .map_err(BaseError::from)?;

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

            accept(obj_slot)
        })
        .await?;

    reservation_val(PageReservation {
        page_id,
        index: page_index,
        image_hash,
        ext: image_ext,
        obj_slot: Some(obj_slot),
    })
}

// Applies the manifest diff and object obligations in one transaction.
async fn apply_manifest<C, R, P, O>(
    (repo, prom, obj_dept, context): (&R, &P, &O, &mut C),
    user_id: &str,
    chapter_id: &str,
    page_specs: &[PageImageSpec],
    page_count: usize,
) -> BaseRest<Vec<PageReservation>>
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

    let retained_ids = page_specs
        .iter()
        .filter_map(|page_spec| page_spec.page_id.clone())
        .collect::<HashSet<_>>();

    let deleted_page_ids = existing_page_infos
        .iter()
        .filter(|page_info| !retained_ids.contains(&page_info.id))
        .map(|page_info| page_info.id.clone())
        .collect::<Vec<_>>();

    ShiftPageIndexesTemporary {
        chapter_id: &chapter_info.id,
    }
    .step_on(repo, context)
    .await?;

    let mut page_entries = Vec::new();

    let mut reservations = Vec::with_capacity(page_count);

    for (index, page_spec) in page_specs.iter().enumerate() {
        //
        let page_info = upsert_page(
            repo,
            context,
            &chapter_info.id,
            &existing_page_infos,
            &mut page_entries,
            index,
            page_spec,
        )
        .await?;

        reservations.push((page_info, page_spec));
    }

    CreatePages {
        entries: &page_entries,
    }
    .step_on(repo, context)
    .await?;

    let mut page_reservations = Vec::with_capacity(page_count);

    for (page_info, page_spec) in reservations {
        //
        let obj_slot =
            reserve_page_obj(obj_dept, context, &page_info, page_spec).await?;

        page_reservations.push(PageReservation {
            page_id: page_info.id,
            index: page_info.index,
            image_hash: page_spec.image_hash.clone(),
            ext: page_spec.ext,
            obj_slot,
        });
    }

    obj_inst! { DelObjs<PageImage>::Remove { ids: &deleted_page_ids } }
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

    accept(page_reservations)
}

// Converts one internal reservation into its response view.
fn reservation_val(reservation: PageReservation) -> BaseRest<ReservedPageVal> {
    //
    let slot = reservation.obj_slot.map(|obj_slot| ImageUploadSlotView {
        put_url: obj_slot.url.to_string(),
        image_version: obj_slot.key.version,
        headers: obj_slot.headers,
    });

    let index = u32::try_from(reservation.index).map_err(|_| {
        //
        BaseError::Unrecoverable {
            message: "page index is out of range".into(),
        }
    })?;

    accept(ReservedPageVal {
        page_id: reservation.page_id,
        index,
        image_hash: reservation.image_hash,
        ext: reservation.ext,
        slot,
    })
}

// Resolves an existing page or stages a newly allocated page.
async fn upsert_page<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    existing_page_infos: &[PageInfo],
    page_entries: &mut Vec<PageEntry>,
    index: usize,
    page_spec: &PageImageSpec,
) -> BaseRest<PageInfo>
where
    C: Context,
    R: PageRepo<C>,
{
    let Some(page_id) = &page_spec.page_id else {
        //
        let page_entry = PageEntry {
            id: PageComplex::gen_id(),
            chapter_id: chapter_id.to_owned(),
            index,
        };

        let page_info = PageInfo {
            id: page_entry.id.clone(),
            chapter_id: page_entry.chapter_id.clone(),
            index,
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
            created_at: time::OffsetDateTime::now_utc(),
            updated_at: time::OffsetDateTime::now_utc(),
        };

        page_entries.push(page_entry);

        return accept(page_info);
    };

    let page_info = existing_page_infos
        .iter()
        .find(|page_info| page_info.id == *page_id)
        .ok_or_else(|| page_not_found(page_id))?;

    let page_update = PageManifestRepl {
        id: page_info.id.clone(),
        index,
    };

    UpdatePageManifest {
        update: &page_update,
    }
    .step_on(repo, context)
    .await
}

// Reuses or reserves the object required by one page specification.
async fn reserve_page_obj<C, O>(
    obj_dept: &O,
    context: &mut C,
    page_info: &PageInfo,
    page_spec: &PageImageSpec,
) -> BaseRest<Option<ObjSlot>>
where
    C: Context + Send,
    O: ObjDept<PageImage, C> + Send + Sync,
{
    let Some(byte_len) = page_spec.new_byte_len else {
        //
        let obj_meta =
            obj_inst! { GetObjMeta<PageImage> { id: &page_info.id } }
                .step_on(obj_dept, context)
                .await
                .map_err(BaseError::from)?;

        ensure_retained_obj(page_info, page_spec, obj_meta.as_ref())?;

        return accept(None);
    };

    let obj_spec = ObjSlotSpec {
        id: &page_info.id,
        hash: page_spec.image_hash.as_bytes(),
        ext: page_spec.ext.suffix(),
        content_type: page_spec.ext.content_type(),
        byte_len,
    };

    let obj_slot = obj_inst! { GenObjSlot<PageImage> { spec: &obj_spec } }
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    accept(Some(obj_slot))
}

// Sums unit counters retained by the new manifest.
fn page_counters(
    page_infos: &[PageInfo],
    retained_ids: &HashSet<String>,
) -> (usize, usize, usize) {
    //
    page_infos
        .iter()
        .filter(|page_info| retained_ids.contains(&page_info.id))
        .fold((0, 0, 0), |counters, page_info| {
            //
            (
                counters.0 + page_info.total_unit_count,
                counters.1 + page_info.translated_unit_count,
                counters.2 + page_info.proofread_unit_count,
            )
        })
}

// Builds the expected missing-page error.
fn page_not_found(page_id: &str) -> BaseError {
    //
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: format!("{}: {}", trl("error-page-not-found"), page_id),
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

    match (
        obj_meta.f_is_uploaded,
        same_hash,
        obj_meta.ext == page_spec.ext.suffix(),
    ) {
        //
        (true, true, true) => accept(()),

        _ => Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: format!("page {} requires a new upload", page_info.id),
        }),
    }
}

// Validates the caller and current chapter state.
async fn ensure_reserve_perm<C, R>(
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
            message: trl("error-page-reserve-role-required"),
        });
    };

    PagePermComplex::ensure_user_can_reserve(&assignment_info)
}
