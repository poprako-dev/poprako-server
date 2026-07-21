//! Page use cases — image reservation, listing, upload confirmation, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::page::{
    ListPageInfosParams, MarkPageImageUploadedParams, PageImageUploadPayload,
    PageInfoVal, ReserveChapterPagesParams, ReserveChapterPagesPayload,
    ReservePageImageParams, ReservedPagePayload,
};
use crate::model::page::PageEntry;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::chapter::CheckUploadFinish;
use crate::part::prom::payload::{Payload, image};
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
    CreatePages, DeletePages, GetPageInfo, ListPageInfos,
    MarkPageImageUploaded, ReservePageImage,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

/// Reserves upload slots for all pages in an empty chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_chapter_pages<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    params: ReserveChapterPagesParams,
) -> BaseResult<ReserveChapterPagesPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let page_count =
        i32::try_from(params.pages.len()).map_err(|_| BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        })?;

    validate_page_count(page_count)?;

    /// Holds the ID, storage key, and version for one reserved page upload.
    struct PageReservation {
        page_id: String,
        index: u32,
        object_key: String,
        image_version: u32,
        image_hash: crate::value::image::ImageHash,
        byte_length: u64,
        extension: crate::value::image::ImageExt,
    }

    PagePermComplex::ensure_user_can_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params.chapter_id,
    )
    .await?;

    let reservations = nucl
        .coord(async move |context| {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &params.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            if chapter_info.page_count != 0 {
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-chapter-pages-already-reserved"),
                });
            }

            let mut page_entries =
                Vec::with_capacity(page_count as usize);

            let mut reservations =
                Vec::with_capacity(page_count as usize);

            for (raw_index, page_input) in params.pages.iter().enumerate() {
                //
                let index = i32::try_from(raw_index).map_err(|_| BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-page-count"),
                })?;

                let page_id = PageComplex::gen_id();

                let image_version = 1;

                let object_key = PageComplex::gen_image_key(
                    &chapter_info.id,
                    &page_id,
                    image_version,
                    page_input.extension.suffix(),
                );

                let page_entry = PageEntry {
                    id: page_id.clone(),
                    chapter_id: chapter_info.id.clone(),
                    index,
                    image_key: Some(object_key.clone()),
                    image_version,
                    image_hash: page_input.image_hash.clone(),
                    image_byte_len: page_input.byte_length,
                    image_ext: page_input.extension,
                };

                page_entries.push(page_entry);

                reservations.push(PageReservation {
                    page_id,
                    index: u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
                        message: "[reserve_chapter_pages] page index must be non-negative".into(),
                    })?,
                    object_key,
                    image_version,
                    image_hash: page_input.image_hash.clone(),
                    byte_length: page_input.byte_length,
                    extension: page_input.extension,
                });
            }

            repo.step(
                context,
                &CreatePages {
                    entries: &page_entries,
                },
            )
            .await?;

            let mut check_ids = Vec::new();

            let mut check_payloads = Vec::new();

            for reservation in &reservations {
                //
                check_ids.push(ImageComplex::gen_check_id());

                check_payloads.push(Payload::Image(
                    image::Payload::CheckUpload {
                        resource_kind: image::ResourceKind::PageImage,
                        resource_id: reservation.page_id.clone(),
                        object_key: reservation.object_key.clone(),
                        version: reservation.image_version,
                    },
                ));
            }

            let check_tasks: Vec<Task<'_, String, Payload>> = check_ids
                .iter()
                .zip(check_payloads.iter())
                .map(|(id, payload)| Task {
                    id,
                    payload,
                    delay: Some(Duration::from_secs(15 * 60)),
                })
                .collect();

            prom.step(context, &DeferBatch::new(&check_tasks)).await?;

            let advance_id = next_snowflake_id();

            let advance_payload =
                Payload::CheckChapterUploadFinish(CheckUploadFinish {
                    chapter_id: chapter_info.id.clone(),
                });

            let advance_task = Task {
                id: &advance_id,
                payload: &advance_payload,
                delay: Some(Duration::from_secs(20 * 60)),
            };

            prom.step(context, &Defer::new(advance_task)).await?;

            repo.step(
                context,
                &SetChapterPageCounters {
                    id: &chapter_info.id,
                    page_count,
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

            accept(reservations)
        })
        .await?;

    let pages = futures_util::future::join_all(reservations.into_iter().map(
        |reservation| async move {
            //
            let put_url = image_pool
                .get_upload_url(&reservation.object_key)
                .await?
                .to_string();

            let mut headers = std::collections::BTreeMap::new();

            headers.insert(
                "content-type".into(),
                reservation.extension.content_type().into(),
            );

            headers.insert(
                "x-amz-checksum-sha256".into(),
                reservation.image_hash.to_base64(),
            );

            accept(ReservedPagePayload {
                page_id: reservation.page_id,
                index: reservation.index,
                image_hash: reservation.image_hash,
                byte_length: reservation.byte_length,
                extension: reservation.extension,
                upload: Some(PageImageUploadPayload {
                    put_url,
                    image_version: reservation.image_version,
                    headers,
                }),
            })
        },
    ))
    .await
    .into_iter()
    .collect::<BaseResult<Vec<_>>>()?;

    accept(ReserveChapterPagesPayload { pages })
}

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
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let page_id = id.clone();

    let page_info = repo.run(&GetPageInfo { id: &id }).await?;

    PagePermComplex::ensure_user_can_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    let file_ext = params.extension.suffix();

    let (object_key, image_version) = nucl
        .coord(async move |context| {
            //
            let page_reservation = repo
                .step(context, &ReservePageImage { id: &id, file_ext })
                .await?;

            let mut batch_ids = Vec::new();

            let mut batch_payloads = Vec::new();

            let mut batch_delays = Vec::new();

            if let Some(prev_object_key) = &page_reservation.prev_object_key
                && prev_object_key != &page_reservation.object_key
            {
                batch_ids.push(ImageComplex::gen_delete_id());

                batch_payloads.push(Payload::Image(image::Payload::Delete {
                    object_key: prev_object_key.clone(),
                }));

                batch_delays.push(None);
            }

            batch_ids.push(ImageComplex::gen_check_id());

            batch_payloads.push(Payload::Image(image::Payload::CheckUpload {
                resource_kind: image::ResourceKind::PageImage,
                resource_id: page_info.id.clone(),
                object_key: page_reservation.object_key.clone(),
                version: page_reservation.image_version,
            }));

            batch_delays.push(Some(Duration::from_secs(15 * 60)));

            let batch_tasks: Vec<Task<'_, String, Payload>> = batch_ids
                .iter()
                .zip(batch_payloads.iter())
                .zip(batch_delays.iter())
                .map(|((id, payload), delay)| Task {
                    id,
                    payload,
                    delay: *delay,
                })
                .collect();

            prom.step(context, &DeferBatch::new(&batch_tasks)).await?;

            accept((
                page_reservation.object_key,
                page_reservation.image_version,
            ))
        })
        .await?;

    let put_url = image_pool.get_upload_url(&object_key).await?.to_string();

    let mut headers = std::collections::BTreeMap::new();

    headers.insert(
        "content-type".into(),
        params.extension.content_type().into(),
    );
    headers.insert(
        "x-amz-checksum-sha256".into(),
        params.image_hash.to_base64(),
    );

    accept(ReservedPagePayload {
        page_id,
        index: u32::try_from(page_info.index).map_err(|_| {
            BaseError::Unrecoverable {
                message: "[reserve_image] page index must be non-negative"
                    .into(),
            }
        })?,
        image_hash: params.image_hash,
        byte_length: params.byte_length,
        extension: params.extension,
        upload: Some(PageImageUploadPayload {
            put_url,
            image_version,
            headers,
        }),
    })
}

/// Lists pages under one chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
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
        .run(&ListPageInfos::Chapter {
            chapter_id: &params.chapter_id,
            offset: params.offset,
            limit: params.limit,
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
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
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

    nucl.coord(async move |context| {
        //
        repo.step(
            context,
            &MarkPageImageUploaded {
                id: &id,
                image_version: params.image_version,
                image_key: None,
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
                &ListPageInfos::AllChapter {
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

/// Validates that the page count is positive.
fn validate_page_count(page_count: i32) -> BaseResult<()> {
    //
    if !(1..=200).contains(&page_count) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        });
    }

    accept(())
}
