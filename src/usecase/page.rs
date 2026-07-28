//! Page use cases — image reservation, listing, upload confirmation, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _, run_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::instr::page::{
    ListPageInfosInstr, MarkPageImageUploadedInstr, ReservePageImageInstr,
};
use crate::data::val::page::{PageInfoVal, ReservedPageVal};
use crate::data::view::image::ImageUploadSlotView;
use crate::model::shared::user::UserToken;
use crate::model::write::page::{PageImageRepl, PageManifestRepl};
use crate::part::image::{ImageManager, ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded, SetChapterPageCounters,
};
use crate::part::repo::oper::comic::{GetComicInfo, TouchComicLastActive};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{
    DeletePages, GetPageInfo, GetPageInfoExcluded, ListPageInfos,
    MarkPageImageUploaded, UpdatePageManifest,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

pub use reserve::reserve_chapter_pages;

// Page reservation workflow and related orchestration.
mod reserve;
#[cfg(test)]
// Unit tests for page metadata and upload reservation flows.
mod tests;

/// Reserves a replacement image upload slot for one page.
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom, image_pool))]
pub async fn reserve_image<N, C, R, P, I>(
    (nucl, repo, prom, image_pool): (&N, &R, &P, &I),
    token: UserToken,
    id: String,
    instr: ReservePageImageInstr,
) -> BaseRest<ReservedPageVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    ImageComplex::ensure_byte_length(
        instr.new_byte_len,
        image::ResourceKind::PageImage,
    )?;

    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    PagePermComplex::ensure_user_can_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

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
                locked_page_info.image_hash == instr.image_hash
                    && locked_page_info.image_ext == instr.ext;

            if same_identity && locked_page_info.is_image_uploaded {
                return accept((locked_page_info, None));
            }

            let (image_key, image_version, prev_image_key) = match same_identity {
                //
                true => (
                    locked_page_info.image_key.clone().ok_or_else(|| {
                        BaseError::Unrecoverable {
                            message: "[reserve_image] pending page image key is missing"
                                .into(),
                        }
                    })?,
                    locked_page_info.image_version,
                    None,
                ),

                false => {
                    //
                    let image_version = locked_page_info
                        .image_version
                        .checked_add(1)
                        .ok_or_else(|| BaseError::Unrecoverable {
                            message: "[reserve_image] image version overflow".into(),
                        })?;

                    let image_key = PageComplex::gen_image_key(
                        &locked_page_info.chapter_id,
                        &locked_page_info.id,
                        image_version,
                        instr.ext.suffix(),
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
                image_hash: instr.image_hash.clone(),
                image_ext: instr.ext,
            };

            let updated_page_info = UpdatePageManifest {
                update: &page_manifest_update,
            }
            .step_on(repo, context)
            .await?;

            let mut task_ids = Vec::new();

            let mut task_payloads = Vec::new();

            let mut task_delays = Vec::new();

            if let Some(prev_image_key) = prev_image_key {
                //
                task_ids.push(ImageComplex::gen_delete_id());

                task_payloads.push(TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: prev_image_key,
                }));

                task_delays.push(None);
            }

            task_ids.push(ImageComplex::gen_check_id());

            task_payloads.push(TaskPayload::Image(image::ImagePayload::CheckUpload {
                resource_kind: image::ResourceKind::PageImage,
                resource_id: locked_page_info.id.clone(),
                object_key: image_key.clone(),
                version: image_version,
            }));

            task_delays.push(Some(Duration::from_secs(15 * 60)));

            let advance_id = next_snowflake_id();

            let advance_payload =
                TaskPayload::Chapter(ChapterPayload::TryAdvanceRawProvideStage {
                    chapter_id: locked_page_info.chapter_id.clone(),
                });

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
            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: page_info.image_ext.content_type(),
                content_length: instr.new_byte_len,
            };

            let upload_target = image_pool.get_upload_slot(upload_spec).await?;

            Some(ImageUploadSlotView {
                put_url: upload_target.url.to_string(),
                image_version: page_info.image_version,
                headers: upload_target.headers,
            })
        }

        None => None,
    };

    accept(ReservedPageVal {
        page_id: page_info.id,
        index: u32::try_from(page_info.index).map_err(|_| {
            BaseError::Unrecoverable {
                message: "[reserve_image] page index must be non-negative"
                    .into(),
            }
        })?,
        image_hash: page_info.image_hash,
        ext: page_info.image_ext,
        slot,
    })
}

/// Lists pages under one chapter.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListPageInfosInstr,
) -> BaseRest<Vec<PageInfoVal>>
where
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    I: ImagePool,
{
    PagePermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &instr.chapter_id,
    )
    .await?;

    let page_infos = ListPageInfos {
        chapter_id: &instr.chapter_id,
    }
    .run_on(repo)
    .await?;

    futures_util::future::join_all(
        page_infos
            .into_iter()
            .map(|page_info| PageInfoVal::from_model(image_pool, page_info)),
    )
    .await
    .into_iter()
    .collect()
}

/// Fetches one page by ID.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn get_info<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    id: String,
) -> BaseRest<PageInfoVal>
where
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    I: ImagePool,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    PagePermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    PageInfoVal::from_model(image_pool, page_info).await
}

/// Marks one page image as uploaded.
#[instrument(level = "info", err(Debug), skip(nucl, repo, image_manager))]
pub async fn mark_image_uploaded<N, C, R, I>(
    (nucl, repo, image_manager): (&N, &R, &I),
    token: UserToken,
    id: String,
    instr: MarkPageImageUploadedInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    I: ImageManager,
{
    let page_info = GetPageInfo { id: &id }.run_on(repo).await?;

    PagePermComplex::ensure_user_can_mark_image_uploaded(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    if page_info.image_version != instr.image_version {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-page-image-upload"),
        });
    }

    if page_info.is_image_uploaded {
        return accept(());
    }

    let image_key =
        page_info
            .image_key
            .clone()
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-page-image-upload"),
            })?;

    if !image_manager.object_exists(&image_key).await? {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-page-image-upload"),
        });
    }

    let repl = PageImageRepl {
        id: id.clone(),
        image_version: instr.image_version,
        image_key: Some(image_key.clone()),
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

        let locked_page_info = GetPageInfoExcluded { id: &id }
            .step_on(repo, context)
            .await?;

        if locked_page_info.image_version != instr.image_version
            || locked_page_info.image_key.as_deref() != Some(&image_key)
        {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-page-image-upload"),
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

/// Deletes all pages under one chapter.
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom))]
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    chapter_id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    PagePermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &chapter_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        let page_infos = ListPageInfos {
            chapter_id: &chapter_info.id,
        }
        .step_on(repo, context)
        .await?;

        let mut delete_ids = Vec::new();

        let mut delete_payloads = Vec::new();

        for page_info in page_infos {
            if let Some(object_key) = page_info.image_key {
                //
                delete_ids.push(ImageComplex::gen_delete_id());

                delete_payloads.push(TaskPayload::Image(
                    image::ImagePayload::Delete { object_key },
                ));
            }
        }

        let delete_tasks = delete_ids
            .iter()
            .zip(delete_payloads.iter())
            .map(|(id, payload)| Task {
                id,
                payload,
                delay: None,
            })
            .collect::<Vec<Task<'_, String, TaskPayload>>>();

        DeferBatch::new(&delete_tasks)
            .step_on(prom, context)
            .await?;

        DeletePages::Chapter {
            chapter_id: &chapter_info.id,
        }
        .step_on(repo, context)
        .await?;

        SetChapterPageCounters {
            id: &chapter_info.id,
            page_count: 0,
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
        }
        .step_on(repo, context)
        .await?;

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
