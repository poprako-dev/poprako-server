//! Complete chapter page-manifest reservation.

// Internal manifest transaction helpers.
mod manifest;

/// Chapter page-count validation.
pub mod validation;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::config::ImageConfig;
use crate::data::instr::page::{
    ReserveChapterPagesInstr, ReservePageImageInstr,
};
use crate::data::val::page::{ReserveChapterPagesVal, ReservedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageImageSpec, PageManifestRepl};
use crate::part::image::{ImagePool, ImageUploadSpec};
use crate::part::nucl::ReptRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::util::collect_bounded;
use crate::usecase::page::reserve::manifest::apply_manifest;
use crate::usecase::page::reserve::validation::validate_page_specs;
use crate::util::next_snowflake_id;
use crate::value::image::{ImageExt, ImageHash, ImageKind};

// Input required to reserve one page-image upload.
struct PageImageReserveSpec {
    //
    // Page identifier being updated.
    page_id: String,
    // Parent chapter identifier.
    chapter_id: String,
    // User initiating the reservation.
    actor_user_id: String,
    // Requested image content hash.
    image_hash: ImageHash,
    // Requested upload size.
    new_byte_len: u64,
    // Requested image extension.
    ext: ImageExt,
}

// Storage keys produced while resolving a page-image identity.
struct PageImageKeys {
    //
    // Active object-storage key.
    image_key: String,
    // Active image version.
    image_version: u32,
    // Previous key scheduled for cleanup.
    prev_image_key: Option<String>,
}

// Page state and upload metadata returned by image reservation.
struct PageImageReserveOutcome {
    //
    // Page state after the reservation update.
    page_info: PageInfo,
    // New upload key, when an upload is required.
    object_key: Option<String>,
    // Requested upload size.
    new_byte_len: u64,
}

/// Reserves upload slots for all pages in an empty chapter.
#[instrument(level = "info", skip(nucl, repo, prom, image_pool, image_config))]
pub async fn reserve_chapter_pages<N, C, R, P, I>(
    (nucl, repo, prom, image_pool, image_config): (
        &N,
        &R,
        &P,
        &I,
        &ImageConfig,
    ),
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
    I: ImagePool + Sync,
{
    let ReserveChapterPagesInstr { chapter_id, pages } = instr;

    let page_specs = pages
        .into_iter()
        .map(PageImageSpec::from)
        .collect::<Vec<_>>();

    let page_count = validate_page_specs(
        image_config,
        &page_specs,
        &chapter_id,
        &token.user_id,
    )?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-reserve-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %token.user_id,
            "expected error: page reservation assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    PagePermComplex::ensure_user_can_reserve(&assignment_info)?;

    let reservations = nucl
        .coord(async move |context| {
            //
            apply_manifest(
                (repo, prom, image_config, context),
                &token.user_id,
                &chapter_id,
                &page_specs,
                page_count,
            )
            .await
        })
        .await?;

    let pages = collect_bounded(reservations.into_iter().map(
        |reservation| async move {
            //
            let (page_id, index, upload, image_version, image_hash, ext) =
                reservation.into_parts();

            let slot = match upload {
                //
                Some(upload) => {
                    //
                    let (object_key, new_byte_len) = upload.into_parts();

                    let upload_spec = ImageUploadSpec {
                        object_key: &object_key,
                        content_type: ext.content_type(),
                        content_length: new_byte_len,
                    };

                    let upload_target =
                        image_pool.get_upload_slot(upload_spec).await?;

                    Some(ImageUploadSlotView {
                        put_url: upload_target.url.to_string(),
                        image_version,
                        headers: upload_target.headers,
                    })
                }

                None => None,
            };

            accept(ReservedPageVal {
                page_id,
                index,
                image_hash,
                ext,
                slot,
            })
        },
    ))
    .await?;

    accept(ReserveChapterPagesVal { pages })
}

/// Reserves a replacement image upload slot for one page.
#[instrument(level = "info", skip(nucl, repo, prom, image_pool, image_config))]
pub async fn reserve_image<N, C, R, P, I>(
    (nucl, repo, prom, image_pool, image_config): (
        &N,
        &R,
        &P,
        &I,
        &ImageConfig,
    ),
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
    I: ImagePool + Sync,
{
    let ReservePageImageInstr {
        image_hash,
        new_byte_len,
        ext,
    } = instr;

    ImageComplex::ensure_byte_length(
        image_config,
        new_byte_len,
        ImageKind::PageImage,
    )?;

    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &page_info.chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-reserve-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            "expected error: page reservation assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    PagePermComplex::ensure_user_can_reserve(&assignment_info)?;

    let reserve_spec = PageImageReserveSpec {
        page_id: id,
        chapter_id: page_info.chapter_id,
        actor_user_id: token.user_id,
        image_hash,
        new_byte_len,
        ext,
    };

    let reservation = nucl
        .coord(async move |context| {
            reserve_page_image(repo, prom, context, reserve_spec).await
        })
        .await?;

    let reserved_page =
        build_reserved_page_val(image_pool, reservation).await?;

    accept(reserved_page)
}

// Reserves a replacement image inside the page transaction.
async fn reserve_page_image<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    spec: PageImageReserveSpec,
) -> BaseRest<PageImageReserveOutcome>
where
    C: Context + Send,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C> + PageRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    // NOTE: Chapter -> Page prevents deadlocks and page-aggregate counter races.
    let chapter_info = GetChapterInfoExcluded {
        id: &spec.chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    ChapterComplex::ensure_chapter_writable(&chapter_info)?;

    let page_info = GetPageInfoExcluded { id: &spec.page_id }
        .step_on(repo, context)
        .await?;

    let same_identity = page_info.image_hash.as_ref() == Some(&spec.image_hash)
        && page_info.image_ext == Some(spec.ext);

    if same_identity && page_info.is_image_uploaded == Some(true) {
        //
        return accept(PageImageReserveOutcome {
            page_info,
            object_key: None,
            new_byte_len: spec.new_byte_len,
        });
    }

    let image_keys =
        resolve_page_image_keys(&page_info, &spec.image_hash, spec.ext)?;

    let page_manifest_update = PageManifestRepl {
        id: page_info.id.clone(),
        index: page_info.index,
        image_key: Some(image_keys.image_key.clone()),
        is_image_uploaded: false,
        image_version: image_keys.image_version,
        image_hash: spec.image_hash,
        image_ext: spec.ext,
    };

    let updated_page_info = UpdatePageManifest {
        update: &page_manifest_update,
    }
    .step_on(repo, context)
    .await?;

    defer_page_image_tasks(
        prom,
        context,
        &updated_page_info,
        &spec.actor_user_id,
        &image_keys,
    )
    .await?;

    accept(PageImageReserveOutcome {
        page_info: updated_page_info,
        object_key: Some(image_keys.image_key),
        new_byte_len: spec.new_byte_len,
    })
}

// Builds the upload-slot response for a page-image reservation.
async fn build_reserved_page_val<I>(
    image_pool: &I,
    reservation: PageImageReserveOutcome,
) -> BaseRest<ReservedPageVal>
where
    I: ImagePool + Sync,
{
    let page_info = reservation.page_info;

    let slot = match reservation.object_key {
        //
        Some(object_key) => {
            //
            let image_ext = require_page_image_value(
                page_info.image_ext,
                "[reserve_image] reserved page image extension is missing",
            )?;

            let image_version = require_page_image_value(
                page_info.image_version,
                "[reserve_image] reserved page image version is missing",
            )?;

            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: image_ext.content_type(),
                content_length: reservation.new_byte_len,
            };

            let upload_target = image_pool.get_upload_slot(upload_spec).await?;

            Some(ImageUploadSlotView {
                put_url: upload_target.url.to_string(),
                image_version,
                headers: upload_target.headers,
            })
        }

        None => None,
    };

    accept(ReservedPageVal {
        page_id: page_info.id,
        index: u32::try_from(page_info.index).map_err(|_| {
            //
            BaseError::Unrecoverable {
                message: "[reserve_image] page index must be non-negative"
                    .into(),
            }
        })?,
        image_hash: require_page_image_value(
            page_info.image_hash,
            "[reserve_image] reserved page image hash is missing",
        )?,
        ext: require_page_image_value(
            page_info.image_ext,
            "[reserve_image] reserved page image extension is missing",
        )?,
        slot,
    })
}

// Resolves the current or next storage key for a page image.
fn resolve_page_image_keys(
    page_info: &PageInfo,
    image_hash: &ImageHash,
    ext: ImageExt,
) -> BaseRest<PageImageKeys> {
    //
    let same_identity = page_info.image_hash.as_ref() == Some(image_hash)
        && page_info.image_ext == Some(ext);

    if same_identity {
        //
        return accept(PageImageKeys {
            image_key: require_page_image_value(
                page_info.image_key.clone(),
                "[reserve_image] pending page image key is missing",
            )?,
            image_version: require_page_image_value(
                page_info.image_version,
                "[reserve_image] pending page image version is missing",
            )?,
            prev_image_key: None,
        });
    }

    let image_version = page_info
        .image_version
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_image] image version overflow".into(),
        })?;

    accept(PageImageKeys {
        image_key: PageComplex::gen_image_key(
            &page_info.chapter_id,
            &page_info.id,
            image_version,
            ext.suffix(),
        ),
        image_version,
        prev_image_key: page_info.image_key.clone(),
    })
}

// Defers storage cleanup, upload verification, and chapter advancement.
async fn defer_page_image_tasks<C, P>(
    prom: &P,
    context: &mut C,
    page_info: &PageInfo,
    actor_user_id: &str,
    image_keys: &PageImageKeys,
) -> BaseRest<()>
where
    C: Context + Send,
    P: Prom<C> + Send + Sync,
{
    let (mut task_ids, mut task_payloads, mut task_delays) =
        (Vec::new(), Vec::new(), Vec::new());

    if let Some(prev_image_key) = &image_keys.prev_image_key {
        //
        task_ids.push(ImageComplex::gen_delete_id());

        task_payloads.push(TaskPayload::Image {
            payload: image::ImagePayload::Delete {
                object_key: prev_image_key.clone(),
            },
        });

        task_delays.push(None);
    }

    task_ids.push(ImageComplex::gen_check_id());

    task_payloads.push(TaskPayload::Image {
        payload: image::ImagePayload::CheckUpload {
            image_kind: ImageKind::PageImage,
            resource_id: page_info.id.clone(),
            object_key: image_keys.image_key.clone(),
            version: image_keys.image_version,
        },
    });

    task_delays.push(Some(Duration::from_mins(15)));

    let image_tasks = task_ids
        .iter()
        .zip(task_payloads.iter())
        .zip(task_delays.iter())
        .map(|((id, payload), delay)| Task {
            id,
            payload,
            delay: *delay,
        })
        .collect::<Vec<Task<'_, String, TaskPayload>>>();

    DeferBatch::new(&image_tasks).step_on(prom, context).await?;

    let advance_id = next_snowflake_id();

    let advance_payload = TaskPayload::Chapter {
        payload: ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id: page_info.chapter_id.clone(),
            actor_user_id: Some(actor_user_id.to_string()),
        },
    };

    let advance_task = Task {
        id: &advance_id,
        payload: &advance_payload,
        delay: Some(Duration::from_mins(20)),
    };

    Defer::new(advance_task).step_on(prom, context).await?;

    accept(())
}

// Requires a persisted page-image value needed by the reservation flow.
fn require_page_image_value<T>(
    value: Option<T>,
    message: &'static str,
) -> BaseRest<T> {
    //
    value.ok_or_else(|| BaseError::Unrecoverable {
        message: message.into(),
    })
}
