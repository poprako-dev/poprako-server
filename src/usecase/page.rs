//! Page use cases — image reservation, listing, upload confirmation, and deletion.

/// Page deletion use case.
pub mod delete;
/// Page read orchestration.
pub mod list;
/// Page reservation workflow and related orchestration.
pub mod reserve;

#[cfg(test)]
// Unit tests for page metadata and upload reservation flows.
mod tests;

use std::time::Duration;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::instr::page::{
    MarkPageImageUploadedInstr, ReservePageImageInstr,
};
use crate::data::val::page::ReservedPageVal;
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageImageRepl, PageManifestRepl};
use crate::part::image::{ImageManager, ImagePool, ImageUploadSpec};
use crate::part::nucl::RepeatableRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::page::{
    GetPageInfo, GetPageInfoExcluded, MarkPageImageUploaded, UpdatePageManifest,
};
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::ImageKind;

/// Reserves a replacement image upload slot for one page.
#[instrument(level = "info", skip(nucl, repo, prom, image_pool))]
pub async fn reserve_image<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    id: String,
    instr: ReservePageImageInstr,
) -> BaseRest<ReservedPageVal>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let ReservePageImageInstr {
        image_hash,
        new_byte_len,
        ext,
    } = instr;

    ImageComplex::ensure_byte_length(new_byte_len, ImageKind::PageImage)?;

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

    let reservation = nucl
        .coord(async move |context| {
            //
            // NOTE: Chapter -> Page is the shared lock order that prevents
            // both deadlocks and page-aggregate counter races.
            GetChapterInfoExcluded {
                id: &page_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await
            .and_then(|chapter_info| {
                ChapterComplex::ensure_chapter_writable(&chapter_info)
            })?;

            let locked_page_info = GetPageInfoExcluded { id: &id }.step_on(repo, context).await?;

            let same_identity =
                locked_page_info.image_hash.as_ref() == Some(&image_hash)
                    && locked_page_info.image_ext == Some(ext);

            if same_identity && locked_page_info.is_image_uploaded == Some(true) {
                return accept((locked_page_info, None));
            }

            let (image_key, image_version, prev_image_key) = match same_identity {
                //
                true => (
                    locked_page_info.image_key.clone().ok_or_else(|| {
                        //
                        BaseError::Unrecoverable {
                            message: "[reserve_image] pending page image key is missing"
                                .into(),
                        }
                    })?,
                    locked_page_info.image_version.ok_or_else(|| {
                        //
                        BaseError::Unrecoverable {
                            message: "[reserve_image] pending page image version is missing".into(),
                        }
                    })?,
                    None,
                ),

                false => {
                    //
                    let image_version = locked_page_info
                        .image_version
                        .unwrap_or(0)
                        .checked_add(1)
                        .ok_or_else(|| BaseError::Unrecoverable {
                            message: "[reserve_image] image version overflow".into(),
                        })?;

                    let image_key = PageComplex::gen_image_key(
                        &locked_page_info.chapter_id,
                        &locked_page_info.id,
                        image_version,
                        ext.suffix(),
                    );

                    (
                        image_key,
                        image_version,
                        locked_page_info.image_key.clone(),
                    )
                }
            };

            let page_manifest_update = PageManifestRepl {
                id: locked_page_info.id.clone(),
                index: locked_page_info.index,
                image_key: Some(image_key.clone()),
                is_image_uploaded: false,
                image_version,
                image_hash,
                image_ext: ext,
            };

            let updated_page_info = UpdatePageManifest {
                update: &page_manifest_update,
            }
            .step_on(repo, context)
            .await?;

            let (mut task_ids, mut task_payloads, mut task_delays) = (
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );

            if let Some(prev_image_key) = prev_image_key {
                //
                task_ids.push(ImageComplex::gen_delete_id());

                task_payloads.push(TaskPayload::Image { payload: image::ImagePayload::Delete {
                    object_key: prev_image_key,
                } });

                task_delays.push(None);
            }

            task_ids.push(ImageComplex::gen_check_id());

            task_payloads.push(TaskPayload::Image { payload: image::ImagePayload::CheckUpload {
                image_kind: ImageKind::PageImage,
                resource_id: locked_page_info.id.clone(),
                object_key: image_key.clone(),
                version: image_version,
            } });

            task_delays.push(Some(Duration::from_secs(15 * 60)));

            let (advance_id, advance_payload) = (
                next_snowflake_id(),
                TaskPayload::Chapter { payload: ChapterPayload::TryAdvanceRawProvideStage {
                    chapter_id: locked_page_info.chapter_id.clone(),
                    actor_user_id: Some(token.user_id.clone()),
                } },
            );

            let advance_task = Task {
                id: &advance_id,
                payload: &advance_payload,
                delay: Some(Duration::from_secs(20 * 60)),
            };

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

            Defer::new(advance_task).step_on(prom, context).await?;

            accept((updated_page_info, Some(image_key)))
        })
        .await?;

    // FIXME: bad taste. No explicit deconstructure of tuples.
    let (page_info, object_key) = reservation;

    let slot = match object_key {
        //
        Some(object_key) => {
            //
            let image_ext = page_info.image_ext.ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[reserve_image] reserved page image extension is missing".into(),
                }
            })?;

            let image_version = page_info.image_version.ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message:
                        "[reserve_image] reserved page image version is missing"
                            .into(),
                }
            })?;

            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: image_ext.content_type(),
                content_length: new_byte_len,
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
        image_hash: page_info.image_hash.ok_or_else(|| {
            //
            BaseError::Unrecoverable {
                message: "[reserve_image] reserved page image hash is missing"
                    .into(),
            }
        })?,
        ext: page_info
            .image_ext
            .ok_or_else(|| BaseError::Unrecoverable {
                message:
                    "[reserve_image] reserved page image extension is missing"
                        .into(),
            })?,
        slot,
    })
}

/// Marks one page image as uploaded.
#[instrument(level = "info", skip(nucl, repo, image_manager))]
pub async fn mark_image_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkPageImageUploadedInstr,
) -> BaseRest<()>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    I: ImageManager,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &page_info.chapter_id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-page-upload-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            "expected error: page upload assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    PagePermComplex::ensure_user_can_mark_image_uploaded(&assignment_info)?;

    if page_info.image_version != Some(instr.image_version) {
        //
        let err_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            page_id = %id,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = page_info.image_version,
            "expected error: stale page image upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if page_info.is_image_uploaded == Some(true) {
        return accept(());
    }

    let image_key = page_info.image_key.clone().ok_or_else(|| {
        //
        let err_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            page_id = %id,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            stored_image_version = page_info.image_version,
            "expected error: stale page image upload",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

    if !image_manager.object_exists(&image_key).await? {
        //
        let err_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            page_id = %id,
            chapter_id = %page_info.chapter_id,
            user_id = %token.user_id,
            image_version = instr.image_version,
            image_key = %image_key,
            "expected error: stale page image upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let repl = PageImageRepl {
        id,
        image_version: instr.image_version,
        image_key: Some(image_key),
        is_image_uploaded: true,
    };

    nucl.coord(async move |context| {
        //
        // NOTE: Chapter -> Page is the shared lock order that prevents both
        // deadlocks and chapter upload-summary races.
        let chapter_info = GetChapterInfoExcluded {
            id: &page_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        let locked_page_info = GetPageInfoExcluded { id: &repl.id }
            .step_on(repo, context)
            .await?;

        if locked_page_info.image_version != Some(instr.image_version)
            || locked_page_info.image_key.as_deref()
                != repl.image_key.as_deref()
        {
            let err_message = trl("error-stale-page-image-upload");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                page_id = %repl.id,
                chapter_id = %page_info.chapter_id,
                user_id = %token.user_id,
                image_version = instr.image_version,
                locked_image_version = locked_page_info.image_version,
                image_key = ?repl.image_key,
                "expected error: stale page image upload",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        MarkPageImageUploaded { repl: &repl }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}
