//! Comic use cases — create, read, update, cover management, and deletion.

use time::{Duration, OffsetDateTime};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::comic::{ComicComplex, ComicPermComplex};
use crate::complex::image::ImageComplex;
use crate::complex::member::MemberPermComplex;
use crate::data::comic::{
    ComicInfoVal, CreateComicData, CreateComicVal, ListComicInfosData, MarkComicCoverUploadedData,
    ReserveComicCoverData, ReserveComicCoverVal, UpdateComicInfoData,
};
use crate::model::comic::{ComicForm, ComicInfoUpdate};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::intention::{IMAGE_TOPIC, ImageIntention, ImageKind};
use crate::part::prom::{Payload, Prom, PromStep, PromTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::proxy::{ProxyNonTransactional, ProxyTransactional};
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub mod tests;

// NOTE: touch_last_active API 不再保留（TODO：删除 note）

/// Creates a new comic inside a workset.
pub async fn create<D, C, R, I>(
    drive: &D,
    repo: &R,
    _image_pool: &I,
    token: UserToken,
    data: CreateComicData,
) -> RootResult<CreateComicVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
    I: ImagePool,
{
    let comic_info = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);
            ComicPermComplex::can_user_create(&mut proxy, &token.user_id, &data.workset_id)
                .await?;

            let index = repo
                .advance(
                    context,
                    &WorksetStep::incr_comic_next_index(&data.workset_id),
                )
                .await?;

            let comic_form = ComicForm {
                id: ComicComplex::gen_id(),
                workset_id: data.workset_id,
                index,
                title: data.title,
                author: data.author,
                description: data.description,
                creator_id: token.user_id,
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

    Ok(CreateComicVal { id: comic_info.id })
}

/// Fetches a comic by ID with cover URL resolution.
pub async fn get_info<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    id: String,
) -> RootResult<ComicInfoVal>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
    I: ImagePool,
{
    let mut proxy = ProxyNonTransactional::new(repo);
    ComicPermComplex::can_user_get_info(&mut proxy, &token.user_id, &id).await?;

    let comic_info = repo.execute(&ComicStep::get_info_by_id(&id)).await?;

    ComicInfoVal::from_model(image_pool, comic_info).await
}

/// Lists comics for a workset.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListComicInfosData,
) -> RootResult<Vec<ComicInfoVal>>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
    I: ImagePool,
{
    let mut proxy = ProxyNonTransactional::new(repo);
    ComicPermComplex::can_user_list_infos(&mut proxy, &token.user_id, &data.workset_id)
        .await?;

    let comic_infos = repo
        .execute(&ComicStep::list_by_workset_id(&data.workset_id))
        .await?;

    let mut comic_info_vals = Vec::with_capacity(comic_infos.len());
    for comic_info in comic_infos {
        // FIXME: join
        comic_info_vals.push(ComicInfoVal::from_model(image_pool, comic_info).await?);
    }

    Ok(comic_info_vals)
}

/// Updates a comic's title, author, and description.
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    data: UpdateComicInfoData,
) -> RootResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
{
    let mut proxy = ProxyNonTransactional::new(repo);
    ComicPermComplex::can_user_update_info(&mut proxy, &token.user_id, &data.id).await?;

    let comic_info_update = ComicInfoUpdate {
        id: data.id,
        title: data.title,
        author: data.author,
        description: data.description,
    };

    repo.execute(&ComicStep::update_info(&comic_info_update))
        .await?;

    Ok(())
}

/// Reserves a new comic cover upload slot.
pub async fn reserve_cover<D, C, R, P, I>(
    drive: &D,
    repo: &R,
    prom: &P,
    image_pool: &I,
    token: UserToken,
    id: String,
    data: ReserveComicCoverData,
) -> RootResult<ReserveComicCoverVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
    I: ImagePool,
{
    let (object_key, cover_version) = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);
            ComicPermComplex::can_user_reserve_cover(&mut proxy, &token.user_id, &id)
                .await?;

            let cover_reservation = repo
                .advance(context, &ComicStep::reserve_cover(&id, &data.file_ext))
                .await?;

            let now = OffsetDateTime::now_utc();

            if let Some(previous_key) = &cover_reservation.previous_object_key {
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
                        object_key: cover_reservation.object_key.clone(),
                        image_version: cover_reservation.cover_version,
                    }),
                    &check_visible_at,
                ),
            )
            .await?;

            accept((
                cover_reservation.object_key,
                cover_reservation.cover_version,
            ))
        })
        .await
        .map_err(map_drive_err)?;

    let put_url = image_pool.put_signed(&object_key).await?.to_string();

    Ok(ReserveComicCoverVal {
        put_url,
        cover_version,
    })
}

/// Marks a reserved comic cover as successfully uploaded.
pub async fn mark_cover_uploaded<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
    data: MarkComicCoverUploadedData,
) -> RootResult<()>
where
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<C> + WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
{
    let mut proxy = ProxyNonTransactional::new(repo);
    ComicPermComplex::can_user_mark_cover_uploaded(&mut proxy, &token.user_id, &id).await?;

    repo.execute(&ComicStep::mark_cover_uploaded(&id, data.cover_version))
        .await?;

    Ok(())
}

/// Deletes a comic and updates the parent workset counter.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);
            ComicPermComplex::can_user_delete(&mut proxy, &token.user_id, &id).await?;

            let comic_info = repo
                .advance(context, &ComicStep::get_info_excluded(&id))
                .await?;
            ComicComplex::delete_cascade(&repo, &prom, context, &comic_info.id).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

/// Marks a comic completed or active.
pub async fn mark_completed<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    id: String,
    is_completed: bool,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
{
    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);
            ComicPermComplex::can_user_mark_completed(&mut proxy, &token.user_id, &id)
                .await?;

            repo.advance(context, &ComicStep::mark_completed(&id, is_completed))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}
