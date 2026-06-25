//! Comic use cases — create, read, update, cover management, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic::ComicComplex;
use crate::complex::image::ImageComplex;
use crate::data::comic::{
    ComicCoverReserveData, ComicCoverReserveVal, ComicCoverUploadedData, ComicCreateData,
    ComicCreateVal, ComicInfoUpdateData, ComicInfoVal, ComicListData,
};
use crate::model::comic::{ComicForm, ComicInfoUpdate};
use crate::part::image::ImagePool;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention, ImageKind};
use crate::part::prom::{Payload, Prom, PromStep, PromTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub mod tests;

/// Creates a new comic inside a workset.
pub async fn create<D, C, R, I>(
    drive: &D,
    repo: &R,
    image: &I,
    data: ComicCreateData,
) -> RootResult<ComicCreateVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + Send,
    I: ImagePool,
{
    let comic_info = drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;

            let index = repo
                .advance(
                    context,
                    // FIXME: excluded?
                    &WorksetStep::increment_comic_next_index(&data.workset_id),
                )
                .await?;

            let comic_form = ComicForm {
                id: ComicComplex::gen_id(),
                workset_id: data.workset_id,
                index,
                title: data.title,
                author: data.author,
                description: data.description,
                creator_id: data.creator_id,
            };

            let comic_info = repo
                .advance(context, &ComicStep::create(&comic_form))
                .await?;

            repo.advance(
                context,
                &WorksetStep::update_comic_count(&comic_form.workset_id, 1),
            )
            .await?;

            accept(comic_info)
        })
        .await
        .map_err(map_drive_err)?;

    Ok(ComicCreateVal {
        comic: ComicInfoVal::from_model(image, comic_info).await?,
    })
}

/// Fetches a comic by ID with cover URL resolution.
pub async fn get_info<C, R, I>(repo: &R, image: &I, id: String) -> RootResult<ComicInfoVal>
where
    R: ComicRepo<C>,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>,
    I: ImagePool,
{
    let comic_info = repo.execute(&ComicStep::get_info_by_id(&id)).await?;

    ComicInfoVal::from_model(image, comic_info).await
}

/// Lists comics for a workset.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image: &I,
    data: ComicListData,
) -> RootResult<Vec<ComicInfoVal>>
where
    R: ComicRepo<C>,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>,
    I: ImagePool,
{
    let comic_infos = repo
        .execute(&ComicStep::list_by_workset_id(&data.workset_id))
        .await?;

    let mut values = Vec::with_capacity(comic_infos.len());
    for comic_info in comic_infos {
        // FIXME: join
        values.push(ComicInfoVal::from_model(image, comic_info).await?);
    }

    Ok(values)
}

/// Updates a comic's title, author, and description.
pub async fn update_info<C, R>(repo: &R, data: ComicInfoUpdateData) -> RootResult<()>
where
    R: ComicRepo<C>,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>,
{
    let update = ComicInfoUpdate {
        id: data.id,
        title: data.title,
        author: data.author,
        description: data.description,
    };

    repo.execute(&ComicStep::update_info(&update)).await?;

    Ok(())
}

/// Reserves a new comic cover upload slot.
pub async fn reserve_cover<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image: &I,
    id: String,
    data: ComicCoverReserveData,
) -> RootResult<ComicCoverReserveVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C> + Send,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, cover_version) = drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let reservation = repo
                .advance(context, &ComicStep::reserve_cover(&id, &data.file_ext))
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(previous_key) = &reservation.previous_object_key {
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageIntention::Delete {
                            object_key: previous_key.clone(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            let check_id = ImageComplex::gen_check_id();
            let check_visible_at = now + Duration::minutes(15);

            prom.advance(
                context,
                &PromStep::append(
                    &check_id,
                    IMAGE_TOPIC,
                    Payload::Image(ImageIntention::CheckUploaded {
                        kind: ImageKind::ComicCover,
                        resource_id: id.clone(),
                        object_key: reservation.object_key.clone(),
                        image_version: reservation.cover_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            accept((reservation.object_key, reservation.cover_version))
        })
        .await
        .map_err(map_drive_err)?;

    let put_url = image.put_signed(&object_key).await?.to_string();

    Ok(ComicCoverReserveVal {
        put_url,
        cover_version,
    })
}

/// Marks a reserved comic cover as successfully uploaded.
pub async fn mark_cover_uploaded<C, R>(
    repo: &R,
    id: String,
    data: ComicCoverUploadedData,
) -> RootResult<()>
where
    R: ComicRepo<C>,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>,
{
    repo.execute(&ComicStep::mark_cover_uploaded(&id, data.cover_version))
        .await?;

    Ok(())
}

/// Deletes a comic and updates the parent workset counter.
pub async fn delete<D, C, R, P>(drive: &D, repo: &R, prom: &P, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;
            let prom = DeriveTransactional::transactional(prom).await;

            let comic = repo
                .advance(context, &ComicStep::get_info_excluded(&id))
                .await?;

            // FIXME: ComicComplex::delete_cascade
            repo.advance(context, &ComicStep::delete(&id)).await?;

            repo.advance(
                context,
                &WorksetStep::update_comic_count(&comic.workset_id, -1),
            )
            .await?;

            if let (true, Some(cover_key)) = (comic.cover_uploaded, &comic.cover_key) {
                let now = OffsetDateTime::now_utc();
                let delete_id = ImageComplex::gen_delete_id();

                prom.advance(
                    context,
                    &PromStep::append(
                        &delete_id,
                        IMAGE_TOPIC,
                        Payload::Image(ImageIntention::Delete {
                            object_key: cover_key.clone(),
                        }),
                        &now,
                    ),
                )
                .await?;
            }

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

/// Marks a comic completed or active.
pub async fn mark_completed<D, C, R>(
    drive: &D,
    repo: &R,
    id: String,
    is_completed: bool,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C> + Send,
{
    drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;

            repo.advance(context, &ComicStep::mark_completed(&id, is_completed))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}
