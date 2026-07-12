//! Page use cases — image reservation, listing, upload confirmation, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::complex::image::ImageComplex;
use crate::complex::page::{PageComplex, PagePermComplex};
use crate::data::page_data;
use crate::model::page_model;
use crate::model::user_model;
use crate::part::image::ImagePool;
use crate::part::prom::task::{IMAGE_TOPIC, ImageKind, ImageTask};
use crate::part::prom::{Payload, Prom, PromStep};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Reserves upload slots for all pages in an empty chapter.
pub async fn reserve_chapter_pages<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: user_model::Token,
    data: page_data::ReserveChapterData,
) -> RegularResult<page_data::ReserveChapterVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    validate_page_count(data.page_count)?;

    /// Holds the ID, storage key, and version for one reserved page upload.
    struct PageReservation {
        page_id: String,
        object_key: String,
        image_version: i64,
    }

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    PagePermComplex::can_user_reserve(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.chapter_id,
    )
    .await?;

    let reservations = drive
        .with_context(
            async move |context| -> RegularResult<Vec<PageReservation>> {
                //
                let repo = repo.derive_transactional().await;

                let chapter_info = repo
                    .advance(
                        context,
                        &ChapterStep::get_info_by_id_excluded(
                            &data.chapter_id,
                            &[],
                        ),
                    )
                    .await?;

                if chapter_info.page_count != 0 {
                    return Err(RegularError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-chapter-pages-already-reserved"),
                    });
                }

                let mut page_forms =
                    Vec::with_capacity(data.page_count as usize);

                let mut reservations =
                    Vec::with_capacity(data.page_count as usize);

                for index in 0..data.page_count {
                    //
                    let page_id = PageComplex::gen_id();

                    let image_version = 1;

                    let object_key = PageComplex::gen_image_key(
                        &chapter_info.id,
                        &page_id,
                        image_version,
                        &data.file_ext,
                    );

                    let page_form = page_model::Form {
                        id: page_id.clone(),
                        chapter_id: chapter_info.id.clone(),
                        index,
                        image_key: Some(object_key.clone()),
                        image_version,
                    };

                    page_forms.push(page_form);

                    reservations.push(PageReservation {
                        page_id,
                        object_key,
                        image_version,
                    });
                }

                repo.advance(context, &PageStep::create_batch(&page_forms))
                    .await?;

                let now = OffsetDateTime::now_utc();

                let check_visible_at = now + Duration::minutes(15);

                for reservation in &reservations {
                    append_check_uploaded(
                        prom,
                        context,
                        &reservation.page_id,
                        &reservation.object_key,
                        reservation.image_version,
                        &check_visible_at,
                    )
                    .await?;
                }

                repo.advance(
                    context,
                    &ChapterStep::set_page_counters(
                        &chapter_info.id,
                        data.page_count,
                        0,
                        0,
                        0,
                    ),
                )
                .await?;

                repo.advance(
                    context,
                    &ComicStep::touch_last_active(&chapter_info.comic_id),
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

            Ok(page_data::CreationVal {
                page_id: reservation.page_id,
                put_url,
                image_version: reservation.image_version,
            })
        }),
    )
    .await
    .into_iter()
    .collect::<RegularResult<Vec<_>>>()?;

    Ok(page_data::ReserveChapterVal { creations })
}

/// Reserves a replacement image upload slot for one page.
pub async fn reserve_image<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: user_model::Token,
    id: String,
    data: page_data::ReserveImageData,
) -> RegularResult<page_data::ReserveImageVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        PageRepoTransactional<C> + AssignmentRepoTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    I: ImagePool,
{
    let page_id = id.clone();

    let page_info = repo.execute(&PageStep::get_info_by_id(&id)).await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    PagePermComplex::can_user_reserve(
        &mut repo.as_proxy(),
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    let file_ext = data.file_ext;

    let (object_key, image_version) = drive
        .with_context(async move |context| -> RegularResult<(String, i64)> {
            //
            let repo = repo.derive_transactional().await;

            let page_reservation = repo
                .advance(context, &PageStep::reserve_image(&id, &file_ext))
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(prev_object_key) = &page_reservation.prev_object_key
                && prev_object_key != &page_reservation.object_key
            {
                append_delete(prom, context, prev_object_key, &now).await?;
            }

            let check_visible_at = now + Duration::minutes(15);

            append_check_uploaded(
                prom,
                context,
                &page_info.id,
                &page_reservation.object_key,
                page_reservation.image_version,
                &check_visible_at,
            )
            .await?;

            Ok((page_reservation.object_key, page_reservation.image_version))
        })
        .await?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(page_data::ReserveImageVal {
        page_id,
        put_url,
        image_version,
    })
}

/// Lists pages under one chapter.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: page_data::ListInfosData,
) -> RegularResult<Vec<page_data::InfoVal>>
where
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Sync,
    <R as DeriveTransactional>::Transactional: PageRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    PagePermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.chapter_id,
    )
    .await?;

    let page_infos = repo
        .execute(&PageStep::list_infos_by_chapter_id(
            &data.chapter_id,
            Page {
                offset: data.offset,
                limit: data.limit,
            },
        ))
        .await?;

    futures_util::future::join_all(
        page_infos.into_iter().map(|page_info| {
            page_data::InfoVal::from_model(image_pool, page_info)
        }),
    )
    .await
    .into_iter()
    .collect()
}

/// Marks one page image as uploaded.
/// TODO: batch
pub async fn mark_image_uploaded<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    id: String,
    data: page_data::MarkImageUploadedData,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: PageRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        PageRepoTransactional<C> + AssignmentRepoTransactional<C> + Send + Sync,
{
    let page_info = repo.execute(&PageStep::get_info_by_id(&id)).await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    PagePermComplex::can_user_mark_image_uploaded(
        &mut repo.as_proxy(),
        &token.user_id,
        &page_info.chapter_id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            repo.advance(
                context,
                &PageStep::mark_image_uploaded(&id, data.image_version),
            )
            .await?;

            Ok(())
        })
        .await?;

    Ok(())
}

/// Deletes all pages under one chapter.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: user_model::Token,
    chapter_id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: PageRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: PageRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    PagePermComplex::can_user_delete(
        &mut repo.as_proxy(),
        &token.user_id,
        &chapter_id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            let chapter_info = repo
                .advance(
                    context,
                    &ChapterStep::get_info_by_id_excluded(&chapter_id, &[]),
                )
                .await?;

            let page_infos = repo
                .advance(
                    context,
                    &PageStep::list_all_infos_by_chapter_id(&chapter_info.id),
                )
                .await?;

            let now = OffsetDateTime::now_utc();

            for page_info in page_infos {
                if let Some(object_key) = page_info.image_key {
                    append_delete(prom, context, &object_key, &now).await?;
                }
            }

            repo.advance(
                context,
                &PageStep::delete_by_chapter_id(&chapter_info.id),
            )
            .await?;

            repo.advance(
                context,
                &ChapterStep::set_page_counters(&chapter_info.id, 0, 0, 0, 0),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
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

// TODO: batch

/// Appends a `CheckUploaded` prom task for the given page image.
async fn append_check_uploaded<C, P>(
    prom: &P,
    context: &mut C,
    page_id: &str,
    object_key: &str,
    image_version: i64,
    visible_at: &OffsetDateTime,
) -> RegularResult<()>
where
    C: Send,
    P: Prom<C> + Send + Sync,
{
    let check_id = ImageComplex::gen_check_id();

    prom.advance(
        context,
        &PromStep::append(
            &check_id,
            IMAGE_TOPIC,
            Payload::Image(ImageTask::CheckUploaded {
                kind: ImageKind::PageImage,
                resource_id: page_id,
                object_key,
                image_version,
            }),
            visible_at,
        ),
    )
    .await
}
/// Appends a `Delete` prom task for the given object key.
async fn append_delete<C, P>(
    prom: &P,
    context: &mut C,
    object_key: &str,
    visible_at: &OffsetDateTime,
) -> RegularResult<()>
where
    C: Send,
    P: Prom<C> + Send + Sync,
{
    let delete_id = ImageComplex::gen_delete_id();

    prom.advance(
        context,
        &PromStep::append(
            &delete_id,
            IMAGE_TOPIC,
            Payload::Image(ImageTask::Delete { object_key }),
            visible_at,
        ),
    )
    .await
}
