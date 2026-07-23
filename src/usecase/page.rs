//! Page use cases — image reservation, listing, upload confirmation, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::DeferBatch;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use self::reserve::validate_image_byte_length;
use crate::complex::chapter::ChapterComplex;
use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::page::{
    ListPageInfosParams, MarkPageImageUploadedParams, PageInfoVal,
    ReservePageImageParams, ReservedPagePayload, PageSlotVal,
};
use crate::model::page::PageManifestUpdate;
use crate::model::user::UserToken;
use crate::part::image::{ImagePool, ImageUploadSpec};
use crate::part::prom::Prom;
use crate::part::prom::payload::{Payload, image};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded, ResetChapterRawProvide,
    SetChapterPageCounters,
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
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

pub use reserve::reserve_chapter_pages;

mod reserve;
#[cfg(test)]
mod tests;

/// Reserves a replacement image upload slot for one page.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_image<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    params: ReservePageImageParams,
) -> BaseResult<ReservedPagePayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    validate_image_byte_length(params.byte_length)?;

    let page_info = repo.run(&GetPageInfo { id: &id }).await?;

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
            repo.step(
                context,
                &GetChapterInfoExcluded {
                    id: &page_info.chapter_id,
                    incls: &[],
                },
            )
            .await
            .and_then(|chapter_info| {
                ChapterComplex::ensure_user_write_allowed(&chapter_info)
            })?;

            let locked_page_info = repo
                .step(context, &GetPageInfoExcluded { id: &id })
                .await?;

            let same_hash = locked_page_info.image_hash == params.image_hash;

            if same_hash
                && (locked_page_info.image_byte_length != params.byte_length
                    || locked_page_info.image_ext != params.ext)
            {
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-page-image-identity"),
                });
            }

            if same_hash && locked_page_info.image_uploaded {
                return accept((locked_page_info, None));
            }

            let (image_key, image_version, previous_image_key) = match same_hash {
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
                        params.ext.suffix(),
                    );

                    (
                        image_key,
                        image_version,
                        locked_page_info.image_key.clone(),
                    )
                }
            };

            let page_manifest_update = PageManifestUpdate {
                id: locked_page_info.id.clone(),
                index: locked_page_info.index,
                image_key: Some(image_key.clone()),
                image_uploaded: false,
                image_version,
                image_hash: params.image_hash.clone(),
                image_byte_len: params.byte_length,
                image_ext: params.ext,
            };

            let updated_page_info = repo
                .step(
                    context,
                    &UpdatePageManifest {
                        update: &page_manifest_update,
                    },
                )
                .await?;

            repo.step(
                context,
                &ResetChapterRawProvide {
                    id: &locked_page_info.chapter_id,
                },
            )
            .await?;

            let mut task_ids = Vec::new();

            let mut task_payloads = Vec::new();

            let mut task_delays = Vec::new();

            if let Some(previous_image_key) = previous_image_key {
                //
                task_ids.push(ImageComplex::gen_delete_id());

                task_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key: previous_image_key,
                }));

                task_delays.push(None);
            }

            task_ids.push(ImageComplex::gen_check_id());

            task_payloads.push(Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::PageImage,
                resource_id: locked_page_info.id.clone(),
                object_key: image_key.clone(),
                version: image_version,
            }));

            task_delays.push(Some(Duration::from_secs(15 * 60)));

            let image_tasks: Vec<Task<'_, String, Payload>> = task_ids
                .iter()
                .zip(task_payloads.iter())
                .zip(task_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&image_tasks)).await?;

            accept((updated_page_info, Some(image_key)))
        })
        .await?;

    let (page_info, object_key) = reservation;

    let slot = match object_key {
        //
        Some(object_key) => {
            //
            let upload_spec = ImageUploadSpec {
                object_key: &object_key,
                content_type: page_info.image_ext.content_type(),
                checksum_sha256: &page_info.image_hash,
                content_length: page_info.image_byte_length,
            };

            let upload_target = image_pool.get_upload_slot(upload_spec).await?;

            Some(PageSlotVal {
                put_url: upload_target.url.to_string(),
                image_version: page_info.image_version,
                headers: upload_target.headers,
            })
        }

        None => None,
    };

    accept(ReservedPagePayload {
        page_id: page_info.id,
        index: u32::try_from(page_info.index).map_err(|_| {
            BaseError::Unrecoverable {
                message: "[reserve_image] page index must be non-negative"
                    .into(),
            }
        })?,
        image_hash: page_info.image_hash,
        byte_length: page_info.image_byte_length,
        ext: page_info.image_ext,
        slot,
    })
}

/// Lists pages under one chapter.
#[instrument(level = "info", err(Debug), skip_all)]
/// Lists all pages for a chapter.
pub async fn list_all_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListPageInfosParams,
) -> BaseResult<Vec<PageInfoVal>>
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
        &params.chapter_id,
    )
    .await?;

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &params.chapter_id,
        })
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

/// Marks one page image as uploaded.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_image_uploaded<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
    params: MarkPageImageUploadedParams,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + PageRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    let page_info = repo.run(&GetPageInfo { id: &id }).await?;

    PagePermComplex::ensure_user_can_mark_image_uploaded(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    if page_info.image_version != params.image_version {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-stale-page-image-upload"),
        });
    }

    let image_key =
        page_info
            .image_key
            .clone()
            .ok_or_else(|| BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-page-image-upload"),
            })?;

    nucl.coord(async move |context| {
        //
        let chapter_info = repo
            .step(
                context,
                &GetChapterInfoExcluded {
                    id: &page_info.chapter_id,
                    incls: &[],
                },
            )
            .await?;

        ChapterComplex::ensure_user_write_allowed(&chapter_info)?;

        let locked_page_info =
            repo.step(context, &GetPageInfoExcluded { id: &id }).await?;

        if locked_page_info.image_version != params.image_version
            || locked_page_info.image_key.as_deref() != Some(&image_key)
            || locked_page_info.image_hash != page_info.image_hash
            || locked_page_info.image_byte_length != page_info.image_byte_length
        {
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: trl("error-stale-page-image-upload"),
            });
        }

        repo.step(
            context,
            &MarkPageImageUploaded {
                id: &id,
                image_version: params.image_version,
                image_key: Some(image_key.as_str()),
            },
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes all pages under one chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    chapter_id: String,
) -> BaseResult<()>
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
        let chapter_info = repo
            .step(
                context,
                &GetChapterInfoExcluded {
                    id: &chapter_id,
                    incls: &[],
                },
            )
            .await?;

        let page_infos = repo
            .step(
                context,
                &ListPageInfos {
                    chapter_id: &chapter_info.id,
                },
            )
            .await?;

        let mut delete_ids = Vec::new();

        let mut delete_payloads = Vec::new();

        for page_info in page_infos {
            if let Some(object_key) = page_info.image_key {
                //
                delete_ids.push(ImageComplex::gen_delete_id());

                delete_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key,
                }));
            }
        }

        let delete_tasks: Vec<Task<'_, String, Payload>> = delete_ids
            .iter()
            .zip(delete_payloads.iter())
            .map(|(id, payload)| Task {
                id,
                payload,
                delay: None,
            })
            .collect();

        prom.step(context, &DeferBatch::new(&delete_tasks)).await?;

        repo.step(
            context,
            &DeletePages::Chapter {
                chapter_id: &chapter_info.id,
            },
        )
        .await?;

        repo.step(
            context,
            &SetChapterPageCounters {
                id: &chapter_info.id,
                page_count: 0,
                total_unit_count: 0,
                translated_unit_count: 0,
                proofread_unit_count: 0,
            },
        )
        .await?;

        repo.step(
            context,
            &TouchComicLastActive {
                id: &chapter_info.comic_id,
            },
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
