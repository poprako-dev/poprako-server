//! Page use cases — image reservation, listing, upload confirmation, and deletion.

use std::time::Duration;

use poprako_orchestra::{Nucl, run_proxy};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::page::ListPageInfosParams;
use crate::data::page::MarkPageImageUploadedParams;
use crate::data::page::PageCreationPayload;
use crate::data::page::PageInfoVal;
use crate::data::page::ReserveChapterPagesParams;
use crate::data::page::ReserveChapterPagesPayload;
use crate::data::page::ReservePageImageParams;
use crate::data::page::ReservePageImagePayload;
use crate::model::page::PageEntry;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
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
use crate::result::{ExpectedVariant, RegularError, RegularResult};

#[cfg(test)]
mod tests;

/// Reserves upload slots for all pages in an empty chapter.
pub async fn reserve_chapter_pages<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    params: ReserveChapterPagesParams,
) -> RegularResult<ReserveChapterPagesPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
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
    validate_page_count(params.page_count)?;

    /// Holds the ID, storage key, and version for one reserved page upload.
    struct PageReservation {
        page_id: String,
        object_key: String,
        image_version: u32,
    }

    PagePermComplex::can_user_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params.chapter_id,
    )
    .await?;

    let reservations = nucl
        .coord(
            async move |context| -> RegularResult<Vec<PageReservation>> {
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
                    return Err(RegularError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-chapter-pages-already-reserved"),
                    });
                }

                let mut page_entries =
                    Vec::with_capacity(params.page_count as usize);

                let mut reservations =
                    Vec::with_capacity(params.page_count as usize);

                for index in 0..params.page_count {
                    //
                    let page_id = PageComplex::gen_id();

                    let image_version = 1;

                    let object_key = PageComplex::gen_image_key(
                        &chapter_info.id,
                        &page_id,
                        image_version,
                        &params.file_ext,
                    );

                    let page_entry = PageEntry {
                        id: page_id.clone(),
                        chapter_id: chapter_info.id.clone(),
                        index,
                        image_key: Some(object_key.clone()),
                        image_version,
                    };

                    page_entries.push(page_entry);

                    reservations.push(PageReservation {
                        page_id,
                        object_key,
                        image_version,
                    });
                }

                repo.step(
                    context,
                    &CreatePages {
                        entries: &page_entries,
                    },
                )
                .await?;

                for reservation in &reservations {
                    append_check_uploaded(
                        prom,
                        context,
                        &reservation.page_id,
                        &reservation.object_key,
                        reservation.image_version,
                    )
                    .await?;
                }

                repo.step(
                    context,
                    &SetChapterPageCounters {
                        id: &chapter_info.id,
                        page_count: params.page_count,
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

                Ok(reservations)
            },
        )
        .await?;

    let creations = futures_util::future::join_all(
        reservations.into_iter().map(|reservation| async move {
            //
            let put_url = image_pool
                .put_signed(&reservation.object_key)
                .await?
                .to_string();

            Ok(PageCreationPayload {
                page_id: reservation.page_id,
                put_url,
                image_version: reservation.image_version,
            })
        }),
    )
    .await
    .into_iter()
    .collect::<RegularResult<Vec<_>>>()?;

    Ok(ReserveChapterPagesPayload { creations })
}

/// Reserves a replacement image upload slot for one page.
pub async fn reserve_image<N, C, R, P, I>(
    nucl: &N,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    params: ReservePageImageParams,
) -> RegularResult<ReservePageImagePayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let page_id = id.clone();

    let page_info = repo.run(&GetPageInfo { id: &id }).await?;

    PagePermComplex::can_user_reserve(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    let file_ext = params.file_ext;

    let (object_key, image_version) = nucl
        .coord(async move |context| -> RegularResult<(String, u32)> {
            let page_reservation = repo
                .step(
                    context,
                    &ReservePageImage {
                        id: &id,
                        file_ext: &file_ext,
                    },
                )
                .await?;

            if let Some(prev_object_key) = &page_reservation.prev_object_key
                && prev_object_key != &page_reservation.object_key
            {
                append_delete(prom, context, prev_object_key).await?;
            }

            append_check_uploaded(
                prom,
                context,
                &page_info.id,
                &page_reservation.object_key,
                page_reservation.image_version,
            )
            .await?;

            Ok((page_reservation.object_key, page_reservation.image_version))
        })
        .await?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReservePageImagePayload {
        page_id,
        put_url,
        image_version,
    })
}

/// Lists pages under one chapter.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListPageInfosParams,
) -> RegularResult<Vec<PageInfoVal>>
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
    PagePermComplex::can_user_list_infos(
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

    let list_page_infos = ListPageInfos::Chapter {
        chapter_id: &params.chapter_id,
        offset: params.offset,
        limit: params.limit,
    };

    let page_infos = repo.run(&list_page_infos).await?;

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
pub async fn mark_image_uploaded<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
    params: MarkPageImageUploadedParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    let page_info = repo.run(&GetPageInfo { id: &id }).await?;

    PagePermComplex::can_user_mark_image_uploaded(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    nucl.coord(async move |context| -> RegularResult<()> {
        repo.step(
            context,
            &MarkPageImageUploaded {
                id: &id,
                image_version: params.image_version,
            },
        )
        .await?;

        Ok(())
    })
    .await?;

    Ok(())
}

/// Deletes all pages under one chapter.
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    chapter_id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
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
    PagePermComplex::can_user_delete(
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

    nucl.coord(async move |context| -> RegularResult<()> {
        let chapter_info = repo
            .step(
                context,
                &GetChapterInfoExcluded {
                    id: &chapter_id,
                    incls: &[],
                },
            )
            .await?;

        let list_page_infos = ListPageInfos::AllChapter {
            chapter_id: &chapter_info.id,
        };

        let page_infos = repo.step(context, &list_page_infos).await?;

        for page_info in page_infos {
            if let Some(object_key) = page_info.image_key {
                append_delete(prom, context, &object_key).await?;
            }
        }

        let delete_pages = DeletePages::Chapter {
            chapter_id: &chapter_info.id,
        };

        repo.step(context, &delete_pages).await?;

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

        Ok(())
    })
    .await?;

    Ok(())
}

/// Validates that the page count is positive.
fn validate_page_count(page_count: i32) -> RegularResult<()> {
    //
    if page_count <= 0 {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-invalid-page-count"),
        });
    }

    Ok(())
}

/// Defers an upload-check task for the given page image.
async fn append_check_uploaded<C, P>(
    prom: &P,
    context: &mut C,
    page_id: &str,
    object_key: &str,
    image_version: u32,
) -> RegularResult<()>
where
    C: Send,
    P: Prom<C> + Send + Sync,
{
    let check_id = ImageComplex::gen_check_id();

    let payload = Payload::Image(image::Payload::CheckUpload {
        resource_kind: image::ResourceKind::PageImage,
        resource_id: page_id.to_string(),
        object_key: object_key.to_string(),
        version: image_version,
    });

    let task = Task {
        id: &check_id,
        payload: &payload,
        delay: Some(Duration::from_secs(15 * 60)),
    };

    prom.step(context, &Defer::new(task)).await
}

/// Defers an image-delete task for the given object key.
async fn append_delete<C, P>(
    prom: &P,
    context: &mut C,
    object_key: &str,
) -> RegularResult<()>
where
    C: Send,
    P: Prom<C> + Send + Sync,
{
    let delete_id = ImageComplex::gen_delete_id();

    let payload = Payload::Image(image::Payload::Delete {
        object_key: object_key.to_string(),
    });

    let task = Task {
        id: &delete_id,
        payload: &payload,
        delay: None,
    };

    prom.step(context, &Defer::new(task)).await
}
